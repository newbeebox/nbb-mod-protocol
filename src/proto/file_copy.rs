use serde::{Deserialize, Serialize};

/// 复制文件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileCopyCommand {
    pub params: Vec<FileCopyParam>,
}

/// 复制文件参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileCopyParam {
    pub input: String,
    pub output: String,
}
