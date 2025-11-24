//! 复制文件命令

use super::super::proto::*;
use super::command_base::{CommandTrait, CommandParams, ProgressCallbackOption};
use super::utils;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl CommandTrait for FileCopyCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;
        let mod_id = params.get_mod_id().await?;

        let mut all_files = Vec::new();
        for item in self.params.iter() {
            let source_path = mod_dir.join(&item.input);
            let target_path = PathBuf::from(utils::env_replace(&params.envs, &item.output)?);

            let files = utils::collect_files(&source_path, Some(&mod_dir))?;
            for (source, _relative) in files {
                let target = if let Ok(rel) = source.strip_prefix(&source_path) {
                    if rel.as_os_str().is_empty() {
                        target_path.clone()
                    } else {
                        target_path.join(rel)
                    }
                } else {
                    target_path.clone()
                };
                all_files.push((source, target));
            }
        }

        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制");
            }
            return Ok(());
        }

        let mut dir_mappings = HashMap::new();

        for (index, (input, output)) in all_files.iter().enumerate() {
            let file_name = input.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(percent, &format!("正在复制: {} ({}/{})", file_name, index + 1, total_files));
            }

            utils::copy_and_record_mapping(input, output, &mod_id, &mut dir_mappings).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(percent, &format!("已复制: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        // 合并并保存所有映射表
        let affected_dirs: Vec<_> = dir_mappings.keys().cloned().collect();
        utils::merge_and_save_mappings(dir_mappings).await?;

        // 整理受影响目录的编号文件
        for dir in affected_dirs {
            utils::reorder_numbered_files(&dir).await?;
        }

        Ok(())
    }

    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let mod_id = params.get_mod_id().await?;

        // 预处理：收集所有路径信息，创建临时存储以保持生命周期
        let target_paths: Vec<_> = self.params
            .iter()
            .map(|item| {
                PathBuf::from(utils::env_replace(&params.envs, &item.output).unwrap_or_default())
            })
            .collect();

        let path_infos: Vec<_> = target_paths
            .iter()
            .map(|target_path| {
                let search_dir = if target_path.is_dir() {
                    target_path.clone()
                } else {
                    target_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
                };
                let file_name = target_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                (file_name, search_dir)
            })
            .collect();

        let files_to_remove = utils::process_removal_mappings(&path_infos, &mod_id).await?;

        let total_files = files_to_remove.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要删除");
            }
            return Ok(());
        }

        for (index, file_path) in files_to_remove.iter().enumerate() {
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(percent, &format!("正在删除: {} ({}/{})", file_name, index + 1, total_files));
            }

            utils::remove_file_and_folder(file_path).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(percent, &format!("已删除: {} ({}/{})", file_name, index + 1, total_files));
            }
        }

        // 整理受影响目录的编号文件
        let unique_dirs: std::collections::HashSet<_> = path_infos.iter().map(|(_, dir)| dir).collect();
        for dir in unique_dirs {
            utils::reorder_numbered_files(dir).await?;
        }

        Ok(())
    }
}
