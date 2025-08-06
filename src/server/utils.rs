use anyhow::anyhow;
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
