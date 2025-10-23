//! 复制文件命令

use crate::server::command_base::{CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;
use anyhow::anyhow;
use tokio::fs;

impl CommandTrait for FileCopyToGameDirCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;
        let mod_dir = params.get_mod_dir()?;

        // 使用工具函数收集所有需要复制的文件
        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &game_dir)?;

        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制");
            }
            return Ok(());
        }

        // 按文件数量计算进度
        for (index, (input, output)) in all_files.iter().enumerate() {
            let percent = ((index * 100) / total_files) as u8;
            let file_name = input.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");
            if let Some(ref callback) = progress {
                callback(percent, &format!("正在复制: {} ({}/{})", file_name, index + 1, total_files));
            }

            // 直接使用原始的 copy 函数，在外层报告进度
            utils::copy(input, output).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已复制: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        Ok(())
    }

    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let game_dir = params.get_game_dir()?;

        // 直接根据协议中的路径参数，在游戏目录中查找要删除的文件
        let mut files_to_remove = Vec::new();
        for path_str in &self.params {
            let path = std::path::Path::new(path_str);

            // 构造目标路径
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(path_str);
            let parent = path.parent().unwrap_or(std::path::Path::new(""));
            let target_dir = game_dir.join(parent);

            // 🔥 从目标目录加载映射表（RenameSort 在目标目录生成映射表）
            let rename_mapping = utils::load_rename_mapping(&target_dir).await;

            // 🔥 应用重命名映射：如果文件被重命名了，使用新名字
            let actual_name = utils::apply_rename_mapping(file_name, &rename_mapping);
            let target_path = target_dir.join(&actual_name);

            // 收集目标位置的所有文件
            if target_path.exists() {
                let files = utils::collect_files(&target_path, None)?;
                for (file, _) in files {
                    if file.exists() && !file.is_dir() {
                        files_to_remove.push(file);
                    }
                }
            }
        }

        // 🔥 去重：避免删除同一个文件多次（多个原文件名映射到同一个新文件名）
        let mut unique_files = std::collections::HashSet::new();
        let files_to_remove: Vec<_> = files_to_remove
            .into_iter()
            .filter(|f| unique_files.insert(f.clone()))
            .collect();

        let total_files = files_to_remove.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要删除");
            }
            return Ok(());
        }

        // 按文件数量计算进度删除
        for (index, file_path) in files_to_remove.iter().enumerate() {
            let percent = ((index * 100) / total_files) as u8;
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");
            if let Some(ref callback) = progress {
                callback(percent, &format!("正在删除: {} ({}/{})", file_name, index + 1, total_files));
            }

            // 直接使用原始的删除函数
            utils::remove_file_and_folder(file_path).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已删除: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        Ok(())
    }
}
