use crate::*;
use anyhow::{Context, anyhow};
use std::path::PathBuf;

/// 命令类型
#[derive(Debug, Clone)]
pub enum CommandType {
    /// 安装
    Install,
    /// 卸载
    UnInstall,
}

#[derive(Debug, Clone)]
pub struct CommandExecuteParams {
    /// 安装|卸载
    pub cmd_type: CommandType,
    /// 环境变量
    pub envs: ModEnvMap,
    /// 模组根路径
    pub mod_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommandParams {
    /// 环境变量
    pub envs: ModEnvMap,
    /// 模组根路径
    pub mod_dir: Option<String>,
}

impl Into<CommandParams> for CommandExecuteParams {
    fn into(self) -> CommandParams {
        CommandParams {
            envs: self.envs.clone(),
            mod_dir: self.mod_dir.clone(),
        }
    }
}

impl CommandParams {
    pub fn from(game_dir: &str, mod_dir: Option<String>) -> Self {
        let mut model = Self {
            envs: ModEnvMap::new(),
            mod_dir,
        };

        model
            .envs
            .insert(ModEnvKey::GameRootDir, game_dir.to_owned());
        model
    }

    pub fn as_cmd_exec_params(&self, cmd_type: CommandType) -> CommandExecuteParams {
        CommandExecuteParams {
            envs: self.envs.clone(),
            mod_dir: self.mod_dir.clone(),
            cmd_type,
        }
    }

    pub fn get_game_dir(&self) -> anyhow::Result<PathBuf> {
        let game_root_dir = self
            .envs
            .get(ModEnvKey::GameRootDir)
            .with_context(|| anyhow!("游戏根路径为空"))?;
        Ok(PathBuf::from(game_root_dir))
    }
    pub fn get_home_dir(&self) -> anyhow::Result<PathBuf> {
        let home_dir = self
            .envs
            .get(ModEnvKey::HomeDir)
            .with_context(|| anyhow!("系统用户根路径为空"))?;
        Ok(PathBuf::from(home_dir))
    }
    pub fn get_mod_dir(&self) -> anyhow::Result<PathBuf> {
        let dir = self
            .mod_dir
            .clone()
            .with_context(|| anyhow!("模组文件路径不存在"))?;
        Ok(PathBuf::from(dir))
    }

    pub fn get_mod_protocol_file(&self) -> anyhow::Result<String> {
        let mod_dir = self.get_mod_dir()?;
        let mod_protocol_file = mod_dir.join("install.json").display().to_string();
        Ok(mod_protocol_file)
    }
}

/// 命令必须实现的方法
pub trait CommandTrait {
    /// 安装
    async fn install(&self, params: CommandParams) -> anyhow::Result<()>;
    /// 卸载
    async fn remove(&self, params: CommandParams) -> anyhow::Result<()>;
    /// 执行安装|卸载
    async fn execute(&self, params: CommandExecuteParams) -> anyhow::Result<()> {
        match params.cmd_type {
            CommandType::Install => self.install(params.into()).await,
            CommandType::UnInstall => self.remove(params.into()).await,
        }
    }
}
