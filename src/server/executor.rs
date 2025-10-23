//！ 命令执行器

use crate::server::command_base::*;
use std::sync::Arc;
use crate::*;
use regex::Regex;
use tokio::fs;


impl Command {
    pub async fn execute(&self, args: CommandExecuteParams, progress: ProgressCallbackOption, all_commands: &[Command]) -> anyhow::Result<()> {
        match self {
            Command::LauncherArg(cmd) => cmd.execute(args, progress, all_commands).await,
            Command::CopyFile(cmd) => cmd.execute(args, progress, all_commands).await,
            Command::CopyToGameDir(cmd) => cmd.execute(args, progress, all_commands).await,
            Command::CopyToHomeDir(cmd) => cmd.execute(args, progress, all_commands).await,
            Command::RenameSort(cmd) => cmd.execute(args, progress, all_commands).await,
        }
    }
}

/// 解析配置文件
async fn parse_command(config_path: &str) -> anyhow::Result<Vec<CommandEntry>> {
    let json_str = fs::read_to_string(config_path).await?;
    if json_str.trim().is_empty() {
        return Err(anyhow::anyhow!("install config is empty:{}", config_path));
    }
    let entries = serde_json::from_str(&json_str)?;
    Ok(entries)
}

/// 安装进度回调函数类型
/// - progress: 进度百分比（0-100）
/// - message: 当前执行的命令描述
pub type InstallProgressCallback = Arc<dyn Fn(u8, &str) + Send + Sync>;

/// 安装模组：返回启动参数
/// - dto: 命令参数
/// - progress_callback: 可选的进度回调函数
pub async fn install(dto: CommandParams, progress_callback: Option<InstallProgressCallback>) -> anyhow::Result<Vec<String>> {
    let config_path = dto.get_mod_protocol_file()?;
    let entries = parse_command(&config_path).await?;

    // 提取所有命令（供命令执行时访问）
    let all_commands: Vec<Command> = entries.iter().map(|e| e.command.clone()).collect();

    // 过滤公有命令
    let public_entries: Vec<&CommandEntry> = entries.iter().filter(|e| e.public).collect();
    let total_commands = public_entries.len();

    let mut args = vec![];
    let dto_clone = dto.clone();

    for (index, entry) in public_entries.iter().enumerate() {
        let base_progress = (index * 100 / total_commands) as u8;
        let next_progress = ((index + 1) * 100 / total_commands) as u8;

        // 获取命令描述
        let command_desc = match &entry.command {
            Command::LauncherArg(_) => "设置启动参数",
            Command::CopyFile(_) => "复制文件",
            Command::CopyToGameDir(_) => "复制文件到游戏目录",
            Command::CopyToHomeDir(_) => "复制文件到用户目录",
            Command::RenameSort(_) => "重命名排序文件",
        };

        // 报告开始执行命令的进度
        if let Some(ref callback) = progress_callback {
            callback(base_progress, &format!("开始{}", command_desc));
        }

        // 启动参数
        if let Command::LauncherArg(cmd) = &entry.command {
            args.extend_from_slice(&cmd.params);
            if let Some(ref callback) = progress_callback {
                callback(next_progress, &format!("完成{}", command_desc));
            }
            continue;
        }

        // 其他命令 - 传递进度回调给命令执行器
        let cmd_params = dto_clone.clone().as_cmd_exec_params(CommandType::Install);

        // 对于非启动参数命令，传递进度回调给命令执行器
        if let Some(ref callback) = progress_callback {
            // 创建子进度回调，将命令内部进度映射到总体进度区间
            let callback_clone = callback.clone();
            let sub_callback: ProgressCallback = Box::new(move |sub_progress: u8, msg: &str| {
                let mapped_progress = base_progress + ((sub_progress as u32 * (next_progress - base_progress) as u32 / 100) as u8);
                callback_clone(mapped_progress, msg);
            });
            entry.command.execute(cmd_params, Some(sub_callback), &all_commands).await?;
        } else {
            entry.command.execute(cmd_params, None, &all_commands).await?;
        }

        // 报告命令完成
        if let Some(ref callback) = progress_callback {
            callback(next_progress, &format!("完成{}", command_desc));
        }
    }

    Ok(args)
}

/// 卸载进度回调函数类型
pub type RemoveProgressCallback = Arc<dyn Fn(u8, &str) + Send + Sync>;

/// 卸载模组
/// - config_path: 配置文件路径
/// - dto: 命令参数
/// - progress_callback: 可选的进度回调函数
pub async fn remove(
    config_path: &str,
    dto: CommandParams,
    progress_callback: Option<RemoveProgressCallback>,
) -> anyhow::Result<()> {
    let entries = parse_command(config_path).await?;

    // 提取所有命令（供命令执行时访问）
    let all_commands: Vec<Command> = entries.iter().map(|e| e.command.clone()).collect();

    // 过滤公有命令
    let public_entries: Vec<&CommandEntry> = entries.iter().filter(|e| e.public).collect();
    let total_commands = public_entries.len();

    let dto_clone = dto.clone();

    for (index, entry) in public_entries.iter().enumerate() {
        let base_progress = (index * 100 / total_commands) as u8;
        let next_progress = ((index + 1) * 100 / total_commands) as u8;

        // 获取命令描述
        let command_desc = match &entry.command {
            Command::LauncherArg(_) => "清除启动参数",
            Command::CopyFile(_) => "删除文件",
            Command::CopyToGameDir(_) => "从游戏目录删除文件",
            Command::CopyToHomeDir(_) => "从用户目录删除文件",
            Command::RenameSort(_) => "重命名排序文件",
        };

        // 报告开始执行命令的进度
        if let Some(ref callback) = progress_callback {
            callback(base_progress, &format!("开始{}", command_desc));
        }

        let cmd_params = dto_clone.clone().as_cmd_exec_params(CommandType::UnInstall);

        // 传递进度回调给命令执行器
        if let Some(ref callback) = progress_callback {
            // 创建子进度回调，将命令内部进度映射到总体进度区间
            let callback_clone = callback.clone();
            let sub_callback: ProgressCallback = Box::new(move |sub_progress: u8, msg: &str| {
                let mapped_progress = base_progress + ((sub_progress as u32 * (next_progress - base_progress) as u32 / 100) as u8);
                callback_clone(mapped_progress, msg);
            });
            entry.command.execute(cmd_params, Some(sub_callback), &all_commands).await?;
        } else {
            entry.command.execute(cmd_params, None, &all_commands).await?;
        }

        // 报告命令完成
        if let Some(ref callback) = progress_callback {
            callback(next_progress, &format!("完成{}", command_desc));
        }
    }

    Ok(())
}
