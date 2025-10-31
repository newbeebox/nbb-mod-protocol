use serde::{Deserialize, Serialize};

/// 文件重命名排序命令
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenameSortCommand {
    pub params: RenameSortParams,
}

/// 重命名排序参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenameSortParams {
    /// 目标目录（支持环境变量如 {{GameRootDir}}）
    pub dir: String,
}
