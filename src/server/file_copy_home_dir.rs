use crate::server::command_base::CommandTrait;
use crate::server::utils;
use crate::*;
use anyhow::anyhow;
use tokio::fs;

impl CommandTrait for FileCopyToHomeDirCommand {
    async fn install(&self, params: CommandParams) -> anyhow::Result<()> {
        let home_dir = params.get_home_dir()?;
        let mod_dir = params.get_mod_dir()?;

        for item in self.params.iter() {
            let input = mod_dir.join(&item);
            let output = home_dir.join(&item);

            utils::copy(&input, &output).await?;
        }
        Ok(())
    }

    async fn remove(&self, params: CommandParams) -> anyhow::Result<()> {
        let home_dir = params.get_home_dir()?;
        for item in self.params.iter() {
            let output = home_dir.join(&item);
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
