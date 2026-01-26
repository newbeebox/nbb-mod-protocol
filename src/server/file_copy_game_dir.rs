//! 复制文件命令

use crate::server::command_base::{CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;

impl CommandTrait for FileCopyToGameDirCommand {
    async fn install(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        _all_commands: &[Command],
    ) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &game_dir)?;
        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制");
            }
            return Ok(());
        }

        for (index, (input, output)) in all_files.iter().enumerate() {
            let file_name = input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("正在复制: {} ({}/{})", file_name, index + 1, total_files),
                );
            }

            utils::copy_file(input, output).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("已复制: {} ({}/{})", file_name, index + 1, total_files),
                );
            }
        }

        Ok(())
    }

    async fn remove(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        _all_commands: &[Command],
    ) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        // 根据命令参数计算目标文件路径
        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &game_dir)?;
        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要删除");
            }
            return Ok(());
        }

        for (index, (_input, output)) in all_files.iter().enumerate() {
            let file_name = output
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("正在删除: {} ({}/{})", file_name, index + 1, total_files),
                );
            }

            utils::remove_file_and_folder(output).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("已删除: {} ({}/{})", file_name, index + 1, total_files),
                );
            }
        }

        Ok(())
    }
}
