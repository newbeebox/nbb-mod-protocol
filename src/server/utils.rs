use crate::proto::{Command, ModEnvKey, ModEnvMap};
use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// 文件映射信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMappingInfo {
    /// 模组ID（协议文件的SHA256哈希）
    pub mod_id: String,
    /// 重命名后的文件名
    pub new_name: String,
}

/// 缓存的环境变量正则表达式
static ENV_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{(\w+)}}").unwrap());

/// 替换环境变量
pub fn env_replace(env: &ModEnvMap, input: &str) -> anyhow::Result<String> {
    let path = ENV_REGEX
        .replace_all(input, |caps: &regex::Captures| {
            let key = &caps[1];
            env.get(ModEnvKey::from(key))
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();
    Ok(path)
}

/// 删除文件和文件夹（如果为空文件夹）
pub async fn remove_file_and_folder(path: &PathBuf) -> anyhow::Result<()> {
    fs::remove_file(path).await?;
    // 然后递归删除空的父目录
    let mut parent = path.parent();
    while let Some(dir) = parent {
        // 尝试读取目录，如果为空则删除
        if let Ok(mut entries) = fs::read_dir(dir).await {
            if entries.next_entry().await?.is_none() {
                fs::remove_dir(dir).await?;
                parent = dir.parent();
                continue;
            }
        }
        break;
    }
    Ok(())
}

// 复制文件
pub async fn copy(input: &PathBuf, output: &PathBuf) -> anyhow::Result<()> {
    // 校验输入文件是否存在
    if !input.exists() {
        return Err(anyhow!("文件不存在：{}", input.display()));
    }

    if input.is_dir() {
        return Ok(());
    }

    // 确保输出目录存在
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| anyhow!("创建文件夹失败:{err:?}"))?;
    }

    // 如果文件存在
    if output.exists() {
        fs::remove_file(output.clone())
            .await
            .map_err(|err| anyhow!("删除失败:{err:?}"))?;
    }

    // 复制文件
    fs::copy(input, output)
        .await
        .map_err(|err| anyhow!("复制文件失败:{err:?}"))?;

    Ok(())
}

/// 收集指定路径下的所有文件
/// 如果是文件，返回单个文件
/// 如果是目录，递归收集所有文件
/// 返回: Vec<(source_path, relative_path)>
pub fn collect_files(path: &Path, base_dir: Option<&Path>) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = Vec::new();

    if path.is_file() {
        // 如果是文件，直接返回
        let relative = if let Some(base) = base_dir {
            path.strip_prefix(base)?.to_path_buf()
        } else {
            path.file_name()
                .ok_or_else(|| anyhow!("无法获取文件名: {}", path.display()))?
                .into()
        };
        files.push((path.to_path_buf(), relative));
    } else if path.is_dir() {
        // 如果是目录，遍历所有文件
        let base = base_dir.unwrap_or(path.parent().unwrap_or(path));
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.path().is_file() {
                let relative = entry.path().strip_prefix(base)?.to_path_buf();
                files.push((entry.path().to_path_buf(), relative));
            }
        }
    }

    Ok(files)
}

/// 收集多个路径下的所有文件并生成源-目标映射
/// 返回: Vec<(source_path, target_path)>
pub fn collect_files_with_mapping(
    items: &[String],
    source_dir: &Path,
    target_dir: &Path,
) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut all_files = Vec::new();

    for item in items {
        let source_path = source_dir.join(item);
        let files = collect_files(&source_path, Some(source_dir))?;

        for (source, relative) in files {
            let target = target_dir.join(&relative);
            all_files.push((source, target));
        }
    }

    Ok(all_files)
}

/// 计算文件 SHA256 哈希值（流式读取，内存占用恒定 8KB）
pub async fn calculate_file_hash(file_path: &Path) -> anyhow::Result<String> {
    use tokio::io::AsyncReadExt;

    let mut file = fs::File::open(file_path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// 加载重命名映射表
/// 从指定目录读取 .nbmod_mapping
/// 返回 original_name -> Vec<FileMappingInfo> 的映射
pub async fn load_rename_mapping(dir: &Path) -> HashMap<String, Vec<FileMappingInfo>> {
    let mapping_file = dir.join(".nbmod_mapping");

    if !mapping_file.exists() {
        return HashMap::new();
    }

    match fs::read_to_string(&mapping_file).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// 复制文件并记录到映射表
///
/// 返回：是否实际执行了文件操作（None表示跳过）
pub async fn copy_and_record_mapping(
    input: &Path,
    output: &Path,
    mod_id: &str,
    dir_mappings: &mut HashMap<PathBuf, HashMap<String, Vec<FileMappingInfo>>>,
) -> anyhow::Result<()> {
    let (actual_path, original_name) = match copy_with_conflict_resolution(input, output).await? {
        Some((path, name, _renamed)) => (path, name),
        None => {
            let name = output.file_name().unwrap().to_str().unwrap().to_string();
            (output.to_path_buf(), name)
        }
    };

    let target_dir = output.parent().unwrap_or(Path::new("")).to_path_buf();
    let actual_name = actual_path.file_name().unwrap().to_str().unwrap().to_string();

    dir_mappings
        .entry(target_dir)
        .or_insert_with(HashMap::new)
        .entry(original_name)
        .or_insert_with(Vec::new)
        .push(FileMappingInfo {
            mod_id: mod_id.to_string(),
            new_name: actual_name,
        });

    Ok(())
}

/// 处理卸载时的映射表更新
///
/// 核心逻辑：
/// 1. 加载所有唯一目录的映射表（并发）
/// 2. 从映射表中移除当前mod_id的记录
/// 3. 收集要删除的文件路径
/// 4. 清理空键并保存更新后的映射表
///
/// 返回：需要删除的文件路径集合
pub async fn process_removal_mappings(
    path_infos: &[(&str, PathBuf)],
    mod_id: &str,
) -> anyhow::Result<std::collections::HashSet<PathBuf>> {
    use futures::future::join_all;

    // 收集唯一目录
    let unique_dirs: std::collections::HashSet<_> = path_infos.iter().map(|(_, dir)| dir.clone()).collect();

    // 并发加载所有映射表
    let load_tasks: Vec<_> = unique_dirs
        .into_iter()
        .map(|dir| async move {
            let mapping = load_rename_mapping(&dir).await;
            (dir, mapping)
        })
        .collect();

    let mut dir_mappings: HashMap<PathBuf, HashMap<String, Vec<FileMappingInfo>>> =
        join_all(load_tasks).await.into_iter().collect();

    // 处理文件并收集要删除的路径
    let mut files_to_remove = std::collections::HashSet::new();
    for (file_name, search_dir) in path_infos {
        if let Some(rename_mapping) = dir_mappings.get_mut(search_dir) {
            if let Some(mappings) = rename_mapping.get_mut(*file_name) {
                mappings.retain(|mapping| {
                    if mapping.mod_id == mod_id {
                        let actual_path = search_dir.join(&mapping.new_name);
                        if actual_path.exists() {
                            files_to_remove.insert(actual_path);
                        }
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    // 并发清理空键并保存所有映射表
    let save_tasks: Vec<_> = dir_mappings
        .into_iter()
        .map(|(target_dir, mut mapping)| async move {
            mapping.retain(|_, v| !v.is_empty());
            save_rename_mapping(&target_dir, &mapping).await
        })
        .collect();

    let results: Vec<_> = join_all(save_tasks).await;
    for result in results {
        result?;
    }

    Ok(files_to_remove)
}

/// 合并并保存所有映射表（按目录，并发处理）
///
/// 对于每个目录：
/// 1. 加载已有映射表
/// 2. 合并新映射（追加到Vec）
/// 3. 保存更新后的映射表
pub async fn merge_and_save_mappings(
    dir_mappings: HashMap<PathBuf, HashMap<String, Vec<FileMappingInfo>>>,
) -> anyhow::Result<()> {
    use futures::future::join_all;

    // 并发处理每个目录
    let tasks: Vec<_> = dir_mappings
        .into_iter()
        .map(|(target_dir, new_mappings)| async move {
            let mut existing_mappings = load_rename_mapping(&target_dir).await;

            for (file_name, new_infos) in new_mappings {
                existing_mappings
                    .entry(file_name)
                    .or_insert_with(Vec::new)
                    .extend(new_infos);
            }

            save_rename_mapping(&target_dir, &existing_mappings).await
        })
        .collect();

    let results: Vec<_> = join_all(tasks).await;
    for result in results {
        result?;
    }

    Ok(())
}

/// 保存重命名映射表到指定目录
/// 将映射表序列化为 JSON 并保存到 .nbmod_mapping 文件
/// 如果映射表为空，删除已有的映射文件
pub async fn save_rename_mapping(
    dir: &Path,
    mapping: &HashMap<String, Vec<FileMappingInfo>>,
) -> anyhow::Result<()> {
    let mapping_file = dir.join(".nbmod_mapping");

    if mapping.is_empty() {
        // 映射表为空，删除映射文件（如果存在）
        if mapping_file.exists() {
            fs::remove_file(&mapping_file).await?;
        }
        return Ok(());
    }

    // 映射表不为空，保存到文件
    let json = serde_json::to_string_pretty(mapping)?;
    fs::write(&mapping_file, json).await?;
    Ok(())
}

/// 查找下一个可用的文件名（处理重名冲突）
/// 智能识别文件名中的 _数字 模式，递增生成新文件名
///
/// 示例：
/// - `file.pak.patch_011.pak` → `file.pak.patch_012.pak`, `file.pak.patch_013.pak`, ...
/// - `file.pak` → `file_0.pak`, `file_1.pak`, ...
/// - `file_0.pak` → `file_1.pak`, `file_2.pak`, ...
/// - `sm70_100_albm.tex.190820018` → `sm70_101_albm.tex.190820018`, `sm70_102_albm.tex.190820018`, ...
async fn find_next_available_name(target: &Path) -> anyhow::Result<PathBuf> {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    let full_name = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("无法解析文件名: {}", target.display()))?;

    // 查找最后一个 _数字 模式
    let mut last_digit_pos = None;
    let mut last_digit_end = None;

    let chars: Vec<char> = full_name.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '_' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            // 找到 _ 后面跟数字
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            // 记录最后一个匹配位置
            last_digit_pos = Some(start);
            last_digit_end = Some(i);
        } else {
            i += 1;
        }
    }

    let (base_name, suffix, start_num, width) = if let (Some(pos), Some(end)) = (last_digit_pos, last_digit_end) {
        // 有 _数字 模式，提取基础名、当前数字、后缀、原始宽度
        let base = &full_name[..pos];
        let digit_str = &full_name[pos + 1..end];
        let current_num: usize = digit_str.parse().unwrap_or(0);
        let original_width = digit_str.len();
        let suf = &full_name[end..];
        (base, suf, current_num + 1, original_width)
    } else {
        // 没有 _数字 模式，在末尾添加 _0
        (full_name, "", 0, 1)
    };

    // 从 start_num 开始尝试
    for i in start_num..start_num + 1000 {
        let new_name = format!("{}_{:0width$}{}", base_name, i, suffix, width = width);
        let new_path = parent.join(&new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(anyhow!("无法找到可用的文件名（尝试了 1000 次）: {}", target.display()))
}

/// 复制文件，自动处理冲突
///
/// 逻辑：
/// 1. 如果目标不存在 → 直接复制
/// 2. 如果目标存在且内容相同 → 跳过
/// 3. 如果目标存在但内容不同 → 重命名为 filename_0.ext 后复制
///
/// 返回：
/// - None: 跳过（目标已存在且内容相同）
/// - Some((actual_path, original_name, renamed)): 复制成功
///   - actual_path: 实际复制到的路径
///   - original_name: 原始文件名
///   - renamed: 是否重命名
pub async fn copy_with_conflict_resolution(
    source: &Path,
    target: &Path,
) -> anyhow::Result<Option<(PathBuf, String, bool)>> {
    let original_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("无法获取文件名: {}", target.display()))?
        .to_string();

    // 先检查目标是否存在（快速路径：90% 情况下直接返回）
    if !target.exists() {
        copy(&source.to_path_buf(), &target.to_path_buf()).await?;
        return Ok(Some((target.to_path_buf(), original_name, false)));
    }

    // 目标存在，计算 hash 比较（慢速路径：10% 情况）
    let source_hash = calculate_file_hash(source).await?;
    let target_hash = calculate_file_hash(target).await?;

    if source_hash == target_hash {
        return Ok(None); // 相同文件，跳过
    }

    // 不同文件，需要重命名
    let new_target = find_next_available_name(target).await?;
    copy(&source.to_path_buf(), &new_target).await?;
    Ok(Some((new_target, original_name, true)))
}

/// 文件编号信息
#[derive(Debug)]
struct NumberedFileInfo {
    path: PathBuf,
    base_name: String,
    number: usize,
    suffix: String,
    width: usize,
}

/// 解析文件名中的编号信息（识别 _数字 模式）
/// 返回：(base_name, number, suffix, width)
fn parse_numbered_filename(full_name: &str) -> Option<(String, usize, String, usize)> {
    let chars: Vec<char> = full_name.chars().collect();
    let mut last_digit_pos = None;
    let mut last_digit_end = None;

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '_' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            last_digit_pos = Some(start);
            last_digit_end = Some(i);
        } else {
            i += 1;
        }
    }

    if let (Some(pos), Some(end)) = (last_digit_pos, last_digit_end) {
        let base = &full_name[..pos];
        let digit_str = &full_name[pos + 1..end];
        let number: usize = digit_str.parse().ok()?;
        let width = digit_str.len();
        let suffix = &full_name[end..];
        Some((base.to_string(), number, suffix.to_string(), width))
    } else {
        None
    }
}

/// 整理目录下的编号文件，消除数字间隙
///
/// 逻辑：
/// 1. 扫描目录，找到所有 xxx_数字.后缀 格式的文件
/// 2. 按 (base_name, suffix) 分组
/// 3. 对每组按数字排序，检查是否连续
/// 4. 如果有间隙，重新编号为连续序列
///
/// 示例：
/// - 卸载前：file_11, file_13, file_15
/// - 卸载后：file_11, file_12, file_13
pub async fn reorder_numbered_files(dir: &Path) -> anyhow::Result<()> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }

    // 1. 扫描目录（只扫描一层）
    let mut entries = fs::read_dir(dir).await?;
    let mut files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                // 跳过映射文件
                if file_name == ".nbmod_mapping" {
                    continue;
                }

                if let Some((base, num, suffix, width)) = parse_numbered_filename(file_name) {
                    files.push(NumberedFileInfo {
                        path: path.clone(),
                        base_name: base,
                        number: num,
                        suffix,
                        width,
                    });
                }
            }
        }
    }

    if files.is_empty() {
        return Ok(());
    }

    // 2. 按 (base_name, suffix) 分组
    let mut groups: HashMap<(String, String), Vec<NumberedFileInfo>> = HashMap::new();
    for file in files {
        groups
            .entry((file.base_name.clone(), file.suffix.clone()))
            .or_default()
            .push(file);
    }

    // 3. 对每组重新编号
    for ((base, suffix), mut group) in groups {
        if group.len() <= 1 {
            continue;
        }

        // 按数字排序
        group.sort_by_key(|f| f.number);

        // 检查是否连续
        let start_num = group[0].number;
        let width = group[0].width;

        let needs_reorder = group
            .iter()
            .enumerate()
            .any(|(i, f)| f.number != start_num + i);

        if !needs_reorder {
            continue;
        }

        // 执行重新编号：先重命名为临时名，再重命名为最终名
        let mut temp_paths = Vec::new();

        for (i, file) in group.iter().enumerate() {
            let expected_num = start_num + i;
            if file.number != expected_num {
                // 临时名：原文件名 + .reordering
                let temp_name = format!(
                    "{}_{:0width$}{}.reordering",
                    base,
                    file.number,
                    suffix,
                    width = width
                );
                let temp_path = dir.join(&temp_name);
                fs::rename(&file.path, &temp_path).await?;
                temp_paths.push((temp_path, expected_num));
            }
        }

        // 重命名为最终名
        for (temp_path, new_num) in temp_paths {
            let new_name = format!("{}_{:0width$}{}", base, new_num, suffix, width = width);
            let new_path = dir.join(&new_name);
            fs::rename(&temp_path, &new_path).await?;
        }
    }

    Ok(())
}


