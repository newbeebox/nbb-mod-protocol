use serde::{Deserialize, Serialize};

/// 复制文件到系统用户根路径
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileCopyToHomeDirCommand {
    pub params: Vec<String>,
}
