use crate::*;

impl Builder { 
    /// 重命名并排序文件 
    ///
    /// # 参数
    /// - `dir`: 目标目录
    /// - `pattern`: 文件匹配模式
    /// - `format`: 格式字符串（必须包含 {index} 占位符）
    /// - `index_start`: 起始索引
    /// - `index_padding`: 索引填充位数
    pub fn rename_sort(
        &mut self,
        dir: impl Into<String>,
        pattern: impl Into<String>,
        format: impl Into<String>,
        index_start: usize,
        index_padding: usize,
    ) -> anyhow::Result<()> {
        self.add(Command::RenameSort(RenameSortCommand {
            params: RenameSortParams {
                dir: dir.into(),
                pattern: pattern.into(),
                format: format.into(),
                index_start,
                index_padding,
            },
        }));
        Ok(())
    }
}
