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
    /// 文件过滤模式（glob，如 "*.pak"）
    pub pattern: String,
    /// 格式字符串（必须包含 {index} 占位符，如 "MyMod_{index}.pak"）
    pub format: String,
    /// 起始索引（默认1）
    #[serde(default = "default_index_start")]
    pub index_start: usize,
    /// 索引填充位数（默认2，即 01, 02...）
    #[serde(default = "default_index_padding")]
    pub index_padding: usize,
}

fn default_index_start() -> usize {
    1
}

fn default_index_padding() -> usize {
    3
}
