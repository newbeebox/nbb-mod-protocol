//! 协议命令定义

use serde::{Deserialize, Serialize};

mod env;
mod file_copy;
mod file_copy_game_dir;
mod file_copy_home_dir;
mod launcher_arg;
mod rename_sort;

pub use env::*;
pub use file_copy::*;
pub use file_copy_game_dir::*;
pub use file_copy_home_dir::*;
pub use launcher_arg::*;
pub use rename_sort::*;

/// 命令类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method")]
#[non_exhaustive]
pub enum Command {
    /// 复制文件
    #[serde(rename = "copy_file")]
    #[deprecated(
        note = "使用 `FileCopyToGameDirCommand`、`FileCopyToHomeDirCommand` 替代 ,目前仅用于支持旧版本"
    )]
    CopyFile(FileCopyCommand),
    /// 启动游戏参数
    #[serde(rename = "launcher_arg")]
    LauncherArg(LauncherArgCommand),
    /// 复制文件到游戏根目录
    #[serde(rename = "copy_to_game_dir")]
    CopyToGameDir(FileCopyToGameDirCommand),
    /// 复制文件到系统用户根目录
    #[serde(rename = "copy_to_home_dir")]
    CopyToHomeDir(FileCopyToHomeDirCommand),
    /// 重命名排序文件
    #[serde(rename = "rename_sort")]
    RenameSort(RenameSortCommand),
}

/// 命令条目（包含可见性控制）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandEntry {
    /// 命令内容
    #[serde(flatten)]
    pub command: Command,

    /// 是否为公有命令（默认 true）
    /// - true: install/remove 自动执行
    /// - false: 需要业务层手动调用
    #[serde(default = "default_public")]
    pub public: bool,
}

fn default_public() -> bool {
    true
}
