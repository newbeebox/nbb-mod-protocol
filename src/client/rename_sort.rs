use crate::*;

impl Builder {
    /// 重命名并排序文件
    ///
    /// 固定行为：
    /// - 扫描目标目录中的所有文件
    /// - **只处理包含 `_数字` 的文件**，不符合规范的文件保持原样
    /// - 按前缀分组，每组独立处理
    /// - 按修改时间排序（旧到新）
    /// - 重新编号为 prefix_0, prefix_1, prefix_2...
    /// - 自动去除重复文件（相同 SHA256）
    ///
    /// # 参数
    /// - `dir`: 目标目录（支持环境变量如 {{GameRootDir}}）
    pub fn rename_sort(&mut self, dir: impl Into<String>) -> anyhow::Result<()> {
        self.add(Command::RenameSort(RenameSortCommand {
            params: RenameSortParams {
                dir: dir.into(),
            },
        }));
        Ok(())
    }
}
