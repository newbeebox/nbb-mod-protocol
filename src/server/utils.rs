use std::path::PathBuf;
use tokio::fs;

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
