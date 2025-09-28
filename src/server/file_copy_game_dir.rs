//! 复制文件命令

use crate::server::command_base::{CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;
use anyhow::anyhow;
use tokio::fs;

impl CommandTrait for FileCopyToGameDirCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        // 使用工具函数收集所有需要复制的文件
        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &game_dir)?;

        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制");
            }
            return Ok(());
        }

        // 按文件数量计算进度
        for (index, (input, output)) in all_files.iter().enumerate() {
            let percent = ((index * 100) / total_files) as u8;
            let file_name = input.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");
            if let Some(ref callback) = progress {
                callback(percent, &format!("正在复制: {} ({}/{})", file_name, index + 1, total_files));
            }

            // 直接使用原始的 copy 函数，在外层报告进度
            utils::copy(input, output).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已复制: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        Ok(())
    }

    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        // 使用工具函数收集所有可能需要删除的文件
        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &game_dir)?;

        // 过滤出实际存在的文件
        let mut files_to_remove = Vec::new();
        for (_, target) in all_files {
            if target.exists() && !target.is_dir() {
                files_to_remove.push(target);
            }
        }

        let total_files = files_to_remove.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要删除");
            }
            return Ok(());
        }

        // 按文件数量计算进度删除
        for (index, file_path) in files_to_remove.iter().enumerate() {
            let percent = ((index * 100) / total_files) as u8;
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");
            if let Some(ref callback) = progress {
                callback(percent, &format!("正在删除: {} ({}/{})", file_name, index + 1, total_files));
            }

            // 直接使用原始的删除函数
            utils::remove_file_and_folder(file_path).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已删除: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        Ok(())
    }
}
