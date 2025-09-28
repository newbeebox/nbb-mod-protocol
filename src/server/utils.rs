use anyhow::anyhow;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

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
