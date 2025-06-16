use serde::{Deserialize, Serialize};

/// 游戏启动参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherArgCommand {
    pub params: Vec<String>,
}
