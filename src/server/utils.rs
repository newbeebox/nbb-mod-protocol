use crate::proto::{ModEnvKey, ModEnvMap};
use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// 递归删除空的父目录
async fn remove_empty_parents(path: &Path) -> anyhow::Result<()> {
    let mut parent = path.parent();
    while let Some(dir) = parent {
        if let Ok(mut entries) = fs::read_dir(dir).await
            && entries.next_entry().await?.is_none()
        {
            fs::remove_dir(dir).await?;
            parent = dir.parent();
            continue;
        }
        break;
    }
    Ok(())
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

/// 复制文件（如果目标已存在则重命名）
/// 返回实际写入的文件路径
pub async fn copy_file(input: &Path, output: &Path) -> anyhow::Result<PathBuf> {
    if !input.exists() {
        return Err(anyhow!("文件不存在：{}", input.display()));
    }

    if input.is_dir() {
        return Ok(output.to_path_buf());
    }

    // 确保输出目录存在
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).await?;
    }

    // 如果目标文件已存在，生成新文件名
    let actual_output = if output.exists() {
        find_next_available_name(output)
    } else {
        output.to_path_buf()
    };

    fs::copy(input, &actual_output).await?;
    Ok(actual_output)
}

/// 匹配文件名中 _数字 模式的正则表达式
static NUMBER_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"_(\d+)").unwrap());

/// 解析文件名中的 _数字 模式
/// 返回: (前缀, 数字, 后缀, 数字位数)
fn parse_numbered_filename(filename: &str) -> Option<(String, u64, String, usize)> {
    let mut last_match: Option<(usize, usize, u64, usize)> = None;

    for cap in NUMBER_PATTERN.captures_iter(filename) {
        if let (Some(full_match), Some(num_match)) = (cap.get(0), cap.get(1))
            && let Ok(num) = num_match.as_str().parse::<u64>()
        {
            let width = num_match.as_str().len();
            last_match = Some((full_match.start(), full_match.end(), num, width));
        }
    }

    last_match.map(|(start, end, num, width)| {
        let prefix = filename[..start].to_string();
        let suffix = filename[end..].to_string();
        (prefix, num, suffix, width)
    })
}

/// 查找下一个可用的文件名（处理重名冲突）
/// 智能识别文件名中的 _数字 模式，递增生成新文件名
///
/// 示例：
/// - `file.pak.patch_011.pak` → `file.pak.patch_012.pak`, `file.pak.patch_013.pak`, ...
/// - `file_0.pak` → `file_1.pak`, `file_2.pak`, ...
/// - `sm70_100_albm.tex.190820018` → `sm70_101_albm.tex.190820018`, ...
fn find_next_available_name(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new("."));
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if let Some((prefix, num, suffix, width)) = parse_numbered_filename(filename) {
        // 有 _数字 模式，递增数字
        let mut next_num = num + 1;
        loop {
            let new_name = format!("{}_{:0width$}{}", prefix, next_num, suffix, width = width);
            let new_path = parent.join(&new_name);
            if !new_path.exists() {
                return new_path;
            }
            next_num += 1;
        }
    } else {
        // 没有 _数字 模式，添加 _1 后缀
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|s| s.to_str());

        let mut counter = 1u64;
        loop {
            let new_name = if let Some(e) = ext {
                format!("{}_{}.{}", stem, counter, e)
            } else {
                format!("{}_{}", stem, counter)
            };
            let new_path = parent.join(&new_name);
            if !new_path.exists() {
                return new_path;
            }
            counter += 1;
        }
    }
}

/// 删除文件并清理空的父目录
pub async fn remove_file_and_folder(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        fs::remove_file(path).await?;
        let _ = remove_empty_parents(path).await;
    }
    Ok(())
}

/// 收集指定路径下的所有文件
/// 如果是文件，返回单个文件
/// 如果是目录，递归收集所有文件
/// 返回: Vec<(source_path, relative_path)>
pub fn collect_files(path: &Path, base_dir: Option<&Path>) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = Vec::new();

    if path.is_file() {
        let relative = if let Some(base) = base_dir {
            path.strip_prefix(base)?.to_path_buf()
        } else {
            path.file_name()
                .ok_or_else(|| anyhow!("无法获取文件名: {}", path.display()))?
                .into()
        };
        files.push((path.to_path_buf(), relative));
    } else if path.is_dir() {
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
pub fn collect_files_with_targets(
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

/// 收集多个路径下的所有文件并生成源-目标映射（别名）
/// 返回: Vec<(source_path, target_path)>
pub fn collect_files_with_mapping(
    items: &[String],
    source_dir: &Path,
    target_dir: &Path,
) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    collect_files_with_targets(items, source_dir, target_dir)
}

/// 收集目标目录中存在的文件（用于卸载，不需要源目录）
/// 根据 params 中的相对路径，在目标目录中查找实际存在的文件
/// 返回: Vec<PathBuf> - 目标文件路径列表
pub fn collect_target_files(items: &[String], target_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for item in items {
        let target_path = target_dir.join(item);

        if target_path.is_file() {
            files.push(target_path);
        } else if target_path.is_dir() {
            // 递归收集目录中的所有文件
            for entry in WalkDir::new(&target_path).into_iter().flatten() {
                if entry.path().is_file() {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files
}
