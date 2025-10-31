///! 重命名排序命令实现

use crate::server::command_base::{CommandParams, CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;
use anyhow::{anyhow, Context};
use glob::glob;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 文件名解析结果
#[derive(Debug, Clone)]
struct FileNameInfo {
    /// 前缀（如 "9ba626afa44a3aa3.patch"）
    prefix: String,
    /// 索引（如 0, 1, 2...）
    index: Option<usize>,
    /// 后缀（如 ".stream", ""）
    suffix: String,
}

/// 解析文件名：提取 prefix_index.suffix 格式
///
/// **规范**：只处理包含 `_数字` 的文件，不符合规范的返回 None
///
/// 示例：
/// - "9ba626afa44a3aa3.patch_0" -> Some(prefix="9ba626afa44a3aa3.patch", index=0, suffix="")
/// - "9ba626afa44a3aa3.patch_0.stream" -> Some(prefix="9ba626afa44a3aa3.patch", index=0, suffix=".stream")
/// - "myfile.pak" -> None（不包含 `_数字`，不处理）
fn parse_filename(name: &str) -> Option<FileNameInfo> {
    let re = Regex::new(r"^(.+?)_(\d+)((?:\.\w+)*)$").unwrap();

    re.captures(name).map(|caps| {
        FileNameInfo {
            prefix: caps[1].to_string(),
            index: caps[2].parse().ok(),
            suffix: caps.get(3).map_or(String::new(), |m| m.as_str().to_string()),
        }
    })
}

/// 扫描所有文件（排除映射表文件）
fn scan_files(dir: &str) -> anyhow::Result<Vec<PathBuf>> {
    let dir_path = Path::new(dir);

    if !dir_path.exists() {
        return Ok(Vec::new());
    }

    // 扫描所有文件，排除 .rename_mapping
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir_path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| {
            p.is_file() && p.file_name().and_then(|n| n.to_str()) != Some(".rename_mapping")
        })
        .collect();

    // 按修改时间排序
    files.sort_by_key(|f| {
        f.metadata()
            .and_then(|m| m.modified())
            .ok()
    });

    Ok(files)
}

/// 重命名操作
struct RenameOperation {
    /// 原文件路径
    old_path: PathBuf,
    /// 新文件名
    new_name: String,
    /// 文件 hash
    hash: String,
}

/// 按前缀分组，并生成重命名操作
async fn group_and_generate_renames(
    dir: &Path,
    files: Vec<PathBuf>,
    progress: &ProgressCallbackOption,
) -> anyhow::Result<(Vec<RenameOperation>, Vec<PathBuf>)> {
    // 第一步：按 (prefix, suffix) 分组，只处理符合规范的文件
    let mut prefix_groups: HashMap<(String, String), Vec<PathBuf>> = HashMap::new();
    let mut skipped_files = 0;

    for file in files {
        let name = file.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("无效的文件名: {}", file.display()))?;

        // 只处理包含 _数字 的文件
        if let Some(info) = parse_filename(name) {
            let key = (info.prefix, info.suffix);
            prefix_groups.entry(key).or_insert_with(Vec::new).push(file);
        } else {
            skipped_files += 1;
        }
    }

    if let Some(callback) = progress {
        if skipped_files > 0 {
            callback(10, &format!("发现 {} 个前缀组，跳过 {} 个不符合规范的文件", prefix_groups.len(), skipped_files));
        } else {
            callback(10, &format!("发现 {} 个前缀组", prefix_groups.len()));
        }
    }

    // 第二步：对每个前缀组，按修改时间排序，计算 hash 去重，重新编号
    let mut rename_ops = Vec::new();
    let mut duplicate_files = Vec::new();
    let mut total_processed = 0;
    let total_groups = prefix_groups.len();

    for ((prefix, suffix), mut group_files) in prefix_groups {
        // 按修改时间排序
        group_files.sort_by_key(|f| {
            f.metadata()
                .and_then(|m| m.modified())
                .ok()
        });

        // 计算 hash，去重
        let mut seen_hashes = std::collections::HashSet::new();
        let mut index = 0;

        for file in group_files {
            let hash = utils::calculate_file_hash(&file).await?;

            if seen_hashes.contains(&hash) {
                // 重复文件，标记删除
                duplicate_files.push(file);
                continue;
            }
            seen_hashes.insert(hash.clone());

            // 生成新文件名：prefix_index 或 prefix_index.suffix
            let new_name = if suffix.is_empty() {
                format!("{}_{}", prefix, index)
            } else {
                format!("{}_{}{}", prefix, index, suffix)
            };

            rename_ops.push(RenameOperation {
                old_path: file,
                new_name,
                hash,
            });

            index += 1;
        }

        total_processed += 1;
        if let Some(callback) = progress {
            let percent = 10 + ((total_processed * 40) / total_groups) as u8;
            callback(percent, &format!("处理前缀: {}", prefix));
        }
    }

    Ok((rename_ops, duplicate_files))
}

/// 执行重命名和删除操作
async fn execute_renames_and_dedup(
    dir: &Path,
    rename_ops: Vec<RenameOperation>,
    duplicate_files: Vec<PathBuf>,
    progress: &ProgressCallbackOption,
) -> anyhow::Result<HashMap<String, utils::FileMappingInfo>> {
    let total_renames = rename_ops.len();
    let total_duplicates = duplicate_files.len();
    let total_operations = total_renames + total_duplicates;

    if total_operations == 0 {
        if let Some(callback) = progress {
            callback(100, "没有文件需要处理");
        }
        return Ok(HashMap::new());
    }

    let mut completed = 0;
    let mut mapping = HashMap::new();

    // 第一步：删除重复文件
    for (idx, file) in duplicate_files.iter().enumerate() {
        let file_name = file.file_name().unwrap().to_str().unwrap();

        if let Some(callback) = progress {
            let percent = 50 + ((completed * 25) / total_operations) as u8;
            callback(percent, &format!("删除重复: {} ({}/{})", file_name, idx + 1, total_duplicates));
        }

        fs::remove_file(file)
            .await
            .with_context(|| format!("删除重复文件失败: {}", file.display()))?;

        completed += 1;
    }

    // 第二步：执行重命名（直接重命名，不需要两阶段提交）
    for (idx, op) in rename_ops.iter().enumerate() {
        let old_name = op.old_path.file_name().unwrap().to_str().unwrap();

        if let Some(callback) = progress {
            let percent = 50 + ((completed * 50) / total_operations) as u8;
            callback(percent, &format!("重命名: {} -> {} ({}/{})", old_name, op.new_name, idx + 1, total_renames));
        }

        let new_path = dir.join(&op.new_name);

        // 如果新旧名字相同，跳过重命名
        if op.old_path != new_path {
            fs::rename(&op.old_path, &new_path)
                .await
                .with_context(|| format!("重命名失败: {} -> {}", op.old_path.display(), new_path.display()))?;

            // 只记录真正发生重命名的文件
            mapping.insert(
                old_name.to_string(),
                utils::FileMappingInfo {
                    new_name: op.new_name.clone(),
                    hash: op.hash.clone(),
                },
            );
        }

        completed += 1;
    }

    if let Some(callback) = progress {
        callback(100, &format!("成功处理 {} 个文件（重命名: {}, 删除重复: {}）",
            total_renames + total_duplicates, total_renames, total_duplicates));
    }

    Ok(mapping)
}

impl CommandTrait for RenameSortCommand {
    async fn install(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        _all_commands: &[Command],
    ) -> anyhow::Result<()> {
        // 1. 环境变量替换
        let dir = utils::env_replace(&params.envs, &self.params.dir)?;
        let dir_path = Path::new(&dir);

        if !dir_path.exists() {
            return Err(anyhow!("目录不存在: {}", dir));
        }

        // 报告开始处理
        if let Some(ref callback) = progress {
            callback(0, "扫描文件中...");
        }

        // 2. 扫描所有文件
        let files = scan_files(&dir)?;

        if files.is_empty() {
            if let Some(callback) = progress {
                callback(100, "没有匹配的文件");
            }
            return Ok(());
        }

        // 报告开始处理
        if let Some(ref callback) = progress {
            callback(5, &format!("发现 {} 个文件", files.len()));
        }

        // 3. 按前缀分组，生成重命名操作
        let (rename_ops, duplicate_files) = group_and_generate_renames(dir_path, files, &progress).await?;

        if duplicate_files.len() > 0 {
            eprintln!("🔥 检测到 {} 个重复文件（相同内容），将自动删除", duplicate_files.len());
        }

        // 4. 执行重命名和删除
        let mapping = execute_renames_and_dedup(
            dir_path,
            rename_ops,
            duplicate_files,
            &progress,
        )
        .await?;

        // 5. 保存映射表到目标目录（直接覆盖，不合并）
        let mapping_file = dir_path.join(".rename_mapping");
        let json = serde_json::to_string_pretty(&mapping)?;
        fs::write(&mapping_file, json).await?;

        Ok(())
    }

    async fn remove(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        all_commands: &[Command],
    ) -> anyhow::Result<()> {
        // 1. 环境变量替换
        let dir = utils::env_replace(&params.envs, &self.params.dir)?;
        let dir_path = Path::new(&dir);

        if !dir_path.exists() {
            return Ok(());
        }

        // 2. 调用 install 重新扫描、排序、重命名
        self.install(params.clone(), progress, all_commands).await?;

        // 3. 检查是否还有文件，如果没有则删除映射表和空目录
        let files = scan_files(&dir)?;
        if files.is_empty() && dir_path.exists() {
            // 删除映射表
            let mapping_file = dir_path.join(".rename_mapping");
            if mapping_file.exists() {
                fs::remove_file(&mapping_file).await?;
            }

            // 删除空目录
            if let Ok(mut entries) = fs::read_dir(dir_path).await {
                if entries.next_entry().await?.is_none() {
                    fs::remove_dir(dir_path).await?;
                }
            }
        }

        Ok(())
    }
}
