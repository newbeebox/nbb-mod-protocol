use crate::proto::{Command, ModEnvKey, ModEnvMap};
use anyhow::anyhow;
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
    /// 重命名后的文件名
    pub new_name: String,
    /// 文件内容 SHA256 哈希值
    pub hash: String,
}

/// 替换环境变量
pub fn env_replace(env: &ModEnvMap, input: &str) -> anyhow::Result<String> {
    let regex = Regex::new(r"\{\{(\w+)}}")?;

    let path = regex
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

/// 计算文件 SHA256 哈希值
pub async fn calculate_file_hash(file_path: &Path) -> anyhow::Result<String> {
    let content = fs::read(file_path).await?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// 加载重命名映射表
/// 从指定目录读取 rename_mapping.json
/// 返回 original_name -> FileMappingInfo 的映射
pub async fn load_rename_mapping(dir: &Path) -> HashMap<String, FileMappingInfo> {
    let mapping_file = dir.join("rename_mapping.json");

    if !mapping_file.exists() {
        return HashMap::new();
    }

    match fs::read_to_string(&mapping_file).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// 应用重命名映射（查找文件的实际名称）
/// 给定原始文件名，返回可能被重命名后的文件名
pub fn apply_rename_mapping(file_name: &str, mapping: &HashMap<String, FileMappingInfo>) -> String {
    mapping
        .get(file_name)
        .map(|info| info.new_name.clone())
        .unwrap_or_else(|| file_name.to_string())
}

/// 从命令列表中加载所有 RenameSort 的映射表
/// 遍历 all_commands，找到所有 RenameSort 命令，加载其映射表并合并
pub async fn load_rename_mappings_from_commands(
    all_commands: &[Command],
    envs: &ModEnvMap,
) -> HashMap<String, FileMappingInfo> {
    let mut rename_mapping = HashMap::new();

    for command in all_commands {
        if let Command::RenameSort(rename_cmd) = command {
            if let Ok(dir) = env_replace(envs, &rename_cmd.params.dir) {
                let mapping = load_rename_mapping(Path::new(&dir)).await;
                rename_mapping.extend(mapping);
            }
        }
    }

    rename_mapping
}
