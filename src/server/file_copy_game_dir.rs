//! 复制文件命令

use crate::server::command_base::CommandTrait;
use crate::*;
use anyhow::anyhow;
use tokio::fs;
use crate::server::utils;

impl CommandTrait for FileCopyToGameDirCommand {
    async fn install(&self, params: CommandParams) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        for item in self.params.iter() {
            let input = mod_dir.join(&item);
            let output = game_dir.join(&item);

            // 如果是文件夹直接跳过
            if input.is_dir() {
                continue;
            }

            // 校验输入文件是否存在
            if !input.exists() {
                return Err(anyhow!("文件不存在：{}", input.display()));
            }

            // 确保输出目录存在
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).await?;
            }
            // 复制文件
            fs::copy(input, output).await?;
        }
        Ok(())
    }

    async fn remove(&self, params: CommandParams) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        for item in self.params.iter() {
            let output = game_dir.join(&item);
            if output.is_dir() {
                continue;
            }
            if output.exists() == false {
                continue;
            }
            utils::remove_file_and_folder(&output).await?;
        }
        Ok(())
    }
}
