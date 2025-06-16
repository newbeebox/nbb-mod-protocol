use std::path::Path;
use walkdir::WalkDir;

/// 读取文件夹所有文件
pub fn get_files(folder_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>> {
    let folder_path = folder_path.as_ref();
    let mut files = Vec::new();
    for entry in WalkDir::new(folder_path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative_path = entry
                .path()
                .strip_prefix(folder_path)?
                .display()
                .to_string()
                .replace("\\", "/");

            files.push(relative_path);
        }
    }
    Ok(files)
}
