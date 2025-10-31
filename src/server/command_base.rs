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
    /// 模组ID（协议文件SHA256），可选（卸载时从外部提供）
    pub mod_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommandParams {
    /// 环境变量
    pub envs: ModEnvMap,
    /// 模组根路径
    pub mod_dir: Option<String>,
    /// 模组ID（协议文件SHA256），可选（卸载时从外部提供）
    pub mod_id: Option<String>,
}

impl Into<CommandParams> for CommandExecuteParams {
    fn into(self) -> CommandParams {
        CommandParams {
            envs: self.envs.clone(),
            mod_dir: self.mod_dir.clone(),
            mod_id: self.mod_id.clone(),
        }
    }
}

impl CommandParams {
    pub fn from(game_dir: &str, mod_dir: Option<String>) -> Self {
        let mut model = Self {
            envs: ModEnvMap::new(),
            mod_dir,
            mod_id: None,
        };

        model
            .envs
            .insert(ModEnvKey::GameRootDir, game_dir.to_owned());
        model
    }

    /// 设置模组ID（用于卸载时从外部提供）
    pub fn with_mod_id(mut self, mod_id: String) -> Self {
        self.mod_id = Some(mod_id);
        self
    }

    pub fn as_cmd_exec_params(&self, cmd_type: CommandType) -> CommandExecuteParams {
        CommandExecuteParams {
            envs: self.envs.clone(),
            mod_dir: self.mod_dir.clone(),
            mod_id: self.mod_id.clone(),
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

    /// 获取模组ID（协议文件的SHA256哈希）
    /// 优先返回已设置的 mod_id，否则从协议文件计算
    pub async fn get_mod_id(&self) -> anyhow::Result<String> {
        // 如果已经设置了 mod_id，直接返回
        if let Some(ref mod_id) = self.mod_id {
            return Ok(mod_id.clone());
        }

        // 否则从协议文件计算
        use tokio::fs;
        use sha2::{Sha256, Digest};

        let protocol_file = self.get_mod_protocol_file()?;
        let content = fs::read_to_string(&protocol_file).await
            .with_context(|| anyhow!("无法读取协议文件: {}", protocol_file))?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}

/// 从协议文件路径计算模组ID
pub async fn calculate_mod_id_from_file(file_path: &str) -> anyhow::Result<String> {
    use tokio::fs;
    use sha2::{Sha256, Digest};

    let content = fs::read_to_string(file_path).await
        .with_context(|| anyhow!("无法读取协议文件: {}", file_path))?;

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// 进度回调函数类型
/// - progress: 进度百分比（0-100）
/// - message: 当前操作描述
pub type ProgressCallback = Box<dyn Fn(u8, &str) + Send + Sync>;

/// 可选的进度回调
pub type ProgressCallbackOption = Option<ProgressCallback>;

/// 命令必须实现的方法
pub trait CommandTrait {
    /// 安装（可选进度回调）
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption, all_commands: &[Command]) -> anyhow::Result<()>;

    /// 卸载（可选进度回调）
    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption, all_commands: &[Command]) -> anyhow::Result<()>;

    /// 执行安装|卸载（可选进度回调）
    async fn execute(&self, params: CommandExecuteParams, progress: ProgressCallbackOption, all_commands: &[Command]) -> anyhow::Result<()> {
        match params.cmd_type {
            CommandType::Install => self.install(params.into(), progress, all_commands).await,
            CommandType::UnInstall => self.remove(params.into(), progress, all_commands).await,
        }
    }
}
