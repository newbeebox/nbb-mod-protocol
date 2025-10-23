///! 重命名排序命令实现

use crate::server::command_base::{CommandParams, CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;
use anyhow::{anyhow, Context};
use glob::glob;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 扫描并排序文件（仅处理第一层文件）
fn scan_and_sort(
    dir: &str,
    params: &RenameSortParams,
) -> anyhow::Result<Vec<PathBuf>> {
    // 构造 glob 模式
    let pattern = Path::new(dir).join(&params.pattern);
    let pattern_str = pattern
        .to_str()
        .ok_or_else(|| anyhow!("无效的路径: {}", pattern.display()))?;

    // 扫描文件，只保留 dir 的直接子文件（第一层）
    let dir_path = Path::new(dir);
    let mut files: Vec<PathBuf> = glob(pattern_str)?
        .filter_map(Result::ok)
        .filter(|p| p.is_file() && p.parent() == Some(dir_path))
        .collect();

    if files.is_empty() {
        return Ok(files);
    }

    // 按修改时间排序
    files.sort_by_key(|f| {
        f.metadata()
            .and_then(|m| m.modified())
            .ok()
    });

    Ok(files)
}

/// 文件去重信息
struct FileDeduplication {
    /// 唯一文件列表（每个 hash 只保留一个）
    unique_files: Vec<PathBuf>,
    /// 重复文件列表（需要删除）
    duplicate_files: Vec<PathBuf>,
    /// 完整映射表（original_name -> FileMappingInfo）
    mapping: HashMap<String, utils::FileMappingInfo>,
    /// 重命名映射表（current_name -> new_name，用于原子性重命名）
    rename_map: HashMap<String, String>,
}

/// 生成重命名映射（带文件 hash 去重）
async fn generate_mapping_with_dedup(
    dir: &Path,
    files: &[PathBuf],
    params: &RenameSortParams,
    old_mapping: &HashMap<String, utils::FileMappingInfo>,
) -> anyhow::Result<FileDeduplication> {
    // 验证格式字符串包含 {index}
    if !params.format.contains("{index}") {
        return Err(anyhow!(
            "格式字符串必须包含 {{index}} 占位符: {}",
            params.format
        ));
    }

    // 第一步：计算所有文件的 hash，按 hash 分组
    let mut hash_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for file in files {
        let hash = utils::calculate_file_hash(file).await?;
        hash_groups.entry(hash).or_insert_with(Vec::new).push(file.clone());
    }

    // 第二步：为每个 hash 组分配新文件名，记录去重信息
    let mut unique_files = Vec::new();
    let mut duplicate_files = Vec::new();
    let mut mapping = HashMap::new();
    let mut rename_map = HashMap::new();
    let mut index_counter = params.index_start;

    for (hash, mut group_files) in hash_groups {
        // 按修改时间排序（与主排序逻辑一致）
        group_files.sort_by_key(|f| f.metadata().and_then(|m| m.modified()).ok());

        // 第一个文件保留，其他文件标记为重复
        let first_file = group_files.first().unwrap();
        unique_files.push(first_file.clone());

        // 生成新文件名
        let index_str = format!("{:0width$}", index_counter, width = params.index_padding);
        let new_name = params.format.replace("{index}", &index_str);
        index_counter += 1;

        // 通过 hash 从旧映射表查找原始文件名
        let original_name = old_mapping
            .iter()
            .find(|(_, info)| info.hash == hash)
            .map(|(original, _)| original.clone())
            .unwrap_or_else(|| {
                // 首次安装，用第一个文件的当前名作为原始名
                group_files.first().unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            });

        // 保存映射表（original_name -> FileMappingInfo）
        mapping.insert(
            original_name,
            utils::FileMappingInfo {
                new_name: new_name.clone(),
                hash: hash.clone(),
            },
        );

        // 保存重命名映射表（current_name -> new_name）
        let current_name = first_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        rename_map.insert(current_name, new_name);

        // 标记重复文件
        if group_files.len() > 1 {
            duplicate_files.extend(group_files.into_iter().skip(1));
        }
    }

    Ok(FileDeduplication {
        unique_files,
        duplicate_files,
        mapping,
        rename_map,
    })
}

/// 原子性重命名并删除重复文件（两阶段提交）
async fn atomic_rename_and_dedup(
    dir: &Path,
    unique_files: &[PathBuf],
    duplicate_files: &[PathBuf],
    rename_map: &HashMap<String, String>,
    progress: &ProgressCallbackOption,
    start_progress: u8,
) -> anyhow::Result<()> {
    let total_unique = unique_files.len();
    let total_duplicates = duplicate_files.len();
    // 每个唯一文件需要2次操作（重命名到临时+临时到最终），重复文件需要1次操作（删除）
    let total_operations = total_unique * 2 + total_duplicates;

    if total_operations == 0 {
        if let Some(callback) = progress {
            callback(100, "没有文件需要处理");
        }
        return Ok(());
    }

    let mut completed = 0;
    let progress_range = 100 - start_progress;

    // Phase 1: 重命名唯一文件到临时文件（避免冲突）
    let mut temp_mappings = Vec::new();

    for (idx, file) in unique_files.iter().enumerate() {
        let old_name = file.file_name().unwrap().to_str().unwrap();

        if let Some(callback) = progress {
            let percent = start_progress + ((completed * progress_range as usize) / total_operations) as u8;
            callback(percent, &format!("准备重命名: {} ({}/{})", old_name, idx + 1, total_unique));
        }

        // 生成临时文件名
        let temp_name = format!(".tmp_{}", old_name);
        let temp_path = dir.join(&temp_name);

        // 重命名到临时文件
        fs::rename(file, &temp_path)
            .await
            .with_context(|| format!("重命名到临时文件失败: {} -> {}", file.display(), temp_path.display()))?;

        temp_mappings.push((temp_path, rename_map.get(old_name).unwrap().clone()));
        completed += 1;
    }

    // Phase 2: 从临时文件重命名到最终文件名
    for (temp_path, new_name) in temp_mappings.iter() {
        if let Some(callback) = progress {
            let percent = start_progress + ((completed * progress_range as usize) / total_operations) as u8;
            callback(percent, &format!("正在重命名: {}", new_name));
        }

        let final_path = dir.join(new_name);

        fs::rename(temp_path, &final_path)
            .await
            .with_context(|| format!("重命名到最终文件失败: {} -> {}", temp_path.display(), final_path.display()))?;

        completed += 1;
    }

    // Phase 3: 删除重复文件
    for (idx, file) in duplicate_files.iter().enumerate() {
        let file_name = file.file_name().unwrap().to_str().unwrap();

        if let Some(callback) = progress {
            let percent = start_progress + ((completed * progress_range as usize) / total_operations) as u8;
            callback(percent, &format!("删除重复文件: {} ({}/{})", file_name, idx + 1, total_duplicates));
        }

        fs::remove_file(file)
            .await
            .with_context(|| format!("删除重复文件失败: {}", file.display()))?;

        completed += 1;
    }

    if let Some(callback) = progress {
        callback(100, &format!("成功处理 {} 个文件（重命名: {}, 删除重复: {}）",
            total_unique + total_duplicates, total_unique, total_duplicates));
    }

    Ok(())
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

        // 2. 加载旧映射表（用于保留原始文件名）
        let old_mapping = utils::load_rename_mapping(dir_path).await;

        // 3. 扫描并排序文件
        let files = scan_and_sort(&dir, &self.params)?;

        if files.is_empty() {
            if let Some(callback) = progress {
                callback(100, "没有匹配的文件");
            }
            return Ok(());
        }

        // 报告开始计算文件 hash
        if let Some(ref callback) = progress {
            callback(5, &format!("分析 {} 个文件...", files.len()));
        }

        // 4. 生成重命名映射（包含文件 hash 去重，保留原始文件名）
        let dedup_result = generate_mapping_with_dedup(dir_path, &files, &self.params, &old_mapping).await?;

        if dedup_result.duplicate_files.len() > 0 {
            eprintln!("🔥 检测到 {} 个重复文件（相同内容），将自动删除", dedup_result.duplicate_files.len());
        }

        // 5. 原子性重命名并删除重复文件
        atomic_rename_and_dedup(
            dir_path,
            &dedup_result.unique_files,
            &dedup_result.duplicate_files,
            &dedup_result.rename_map,
            &progress,
            10, // 从 10% 开始，前面已经用了 0-10%
        )
        .await?;

        // 6. 保存映射表到目标目录（直接覆盖，不合并）
        let mapping_file = dir_path.join("rename_mapping.json");
        let json = serde_json::to_string_pretty(&dedup_result.mapping)?;
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
        let files = scan_and_sort(&dir, &self.params)?;
        if files.is_empty() && dir_path.exists() {
            // 删除映射表
            let mapping_file = dir_path.join("rename_mapping.json");
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
