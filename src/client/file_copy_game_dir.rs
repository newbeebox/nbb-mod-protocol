use crate::client::utils;
use crate::*;
use std::path::Path;

impl Builder {
    pub fn copy_to_game_dir(&mut self, folder_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let folder_path = folder_path.as_ref();

        if !folder_path.exists() {
            return Err(anyhow::anyhow!("文件夹不存在: {}", folder_path.display()));
        }
        if !folder_path.is_dir() {
            return Err(anyhow::anyhow!("只能传文件夹: {}", folder_path.display()));
        }
        let files = utils::get_files(folder_path)?;
        self.add(Command::CopyToGameDir(FileCopyToGameDirCommand {
            params: files,
        }));

        Ok(())
    }
}
