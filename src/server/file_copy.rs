//! 复制文件命令

use super::super::proto::*;
use super::command_base::*;
use super::utils;
use anyhow::anyhow;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 替换环境变量
fn env_replace(env: &ModEnvMap, input: &str) -> anyhow::Result<String> {
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

impl CommandTrait for FileCopyCommand {
    async fn install(&self, params: CommandParams) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        for item in self.params.iter() {
            let input = mod_dir.join(item.input.clone());
            let output = env_replace(&params.envs, &item.output)?;
            let output = PathBuf::from(&output);
            utils::copy(&input, &output).await?;
        }
        Ok(())
    }

    async fn remove(&self, params: CommandParams) -> anyhow::Result<()> {
        for item in self.params.iter() {
            let output = env_replace(&params.envs, &item.output)?;
            let output = PathBuf::from(output);
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
