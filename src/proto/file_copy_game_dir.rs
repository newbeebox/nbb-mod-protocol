use serde::{Deserialize, Serialize};

/// 复制文件到游戏根路径
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileCopyToGameDirCommand {
    pub params: Vec<String>,
}
