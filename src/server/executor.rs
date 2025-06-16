//！ 命令执行器

use crate::server::command_base::*;
use crate::*;
use regex::Regex;
use tokio::fs;

macro_rules! delegate_execute {
    ($self:ident, $args:ident, $($variant:ident),*) => {
        match $self {
            $(Command::$variant(cmd) => cmd.execute($args).await,)*
        }
    };
}

impl Command {
    pub async fn execute(&self, args: CommandExecuteParams) -> anyhow::Result<()> {
        delegate_execute!(
            self,
            args,
            LauncherArg,
            CopyFile,
            CopyToGameDir,
            CopyToHomeDir
        )
    }
}

/// 解析配置文件
async fn parse_command(config_path: &str) -> anyhow::Result<Vec<Command>> {
    let json_str = fs::read_to_string(config_path).await?;
    if json_str.trim().is_empty() {
        return Err(anyhow::anyhow!("install config is empty:{}", config_path));
    }
    let commands = serde_json::from_str(&json_str)?;
    Ok(commands)
}

/// 安装模组：返回启动参数
pub async fn install(dto: CommandParams) -> anyhow::Result<Vec<String>> {
    let config_path = dto.get_mod_protocol_file()?;
    let commands = parse_command(&config_path).await?;
    let mut args = vec![];
    let dto_clone = dto.clone();
    for command in commands {
        // 启动参数
        if let Command::LauncherArg(cmd) = &command {
            args.extend_from_slice(&cmd.params);
            continue;
        }
        // 其他命令
        let cmd_params = dto_clone.clone().as_cmd_exec_params(CommandType::Install);
        command.execute(cmd_params).await?;
    }

    Ok(args)
}

/// 卸载模组
pub async fn remove(config_path: &str, dto: CommandParams) -> anyhow::Result<()> {
    let commands = parse_command(config_path).await?;
    let dto_clone = dto.clone();

    for command in commands {
        let cmd_params = dto_clone.clone().as_cmd_exec_params(CommandType::UnInstall);
        command.execute(cmd_params).await?;
    }

    Ok(())
}
