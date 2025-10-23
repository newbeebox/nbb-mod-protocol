//! 复制文件命令

use super::super::proto::*;
use super::command_base::{CommandTrait, CommandParams, ProgressCallbackOption};
use super::utils;
use anyhow::anyhow;
use std::path::{Path, PathBuf};
use tokio::fs;

impl CommandTrait for FileCopyCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        // 收集所有需要复制的文件
        let mut all_files = Vec::new();
        for item in self.params.iter() {
            let source_path = mod_dir.join(&item.input);
            let target_path = utils::env_replace(&params.envs, &item.output)?;
            let target_path = PathBuf::from(target_path);

            // 使用工具函数收集文件
            let files = utils::collect_files(&source_path, Some(&mod_dir))?;
            for (source, _relative) in files {
                // 对于每个源文件，计算相对于 input 的路径
                if let Ok(rel) = source.strip_prefix(&source_path) {
                    let target = if rel.as_os_str().is_empty() {
                        target_path.clone()
                    } else {
                        target_path.join(rel)
                    };
                    all_files.push((source, target));
                } else {
                    // 如果是单个文件
                    all_files.push((source, target_path.clone()));
                }
            }
        }

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

            utils::copy(input, output).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已复制: {} ({}/{})", file_name, index + 1, total_files));
            }
        }
        Ok(())
    }

    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        // 收集所有需要删除的文件，直接从协议中的 output 路径删除
        let mut files_to_remove = Vec::new();
        for item in self.params.iter() {
            let target_path = utils::env_replace(&params.envs, &item.output)?;
            let target_path = PathBuf::from(target_path);

            // 🔥 从目标文件所在目录加载映射表（RenameSort 在目标目录生成映射表）
            let search_dir = if target_path.is_dir() {
                target_path.clone()
            } else {
                target_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
            };
            let rename_mapping = utils::load_rename_mapping(&search_dir).await;

            // 🔥 应用重命名映射：替换路径中的文件名
            let file_name = target_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let actual_name = utils::apply_rename_mapping(file_name, &rename_mapping);
            let actual_path = if file_name != actual_name {
                // 文件被重命名了，替换路径中的文件名
                target_path.parent()
                    .map(|p| p.join(&actual_name))
                    .unwrap_or_else(|| PathBuf::from(&actual_name))
            } else {
                target_path.clone()
            };

            // 判断 input 是文件还是目录（通过是否有扩展名简单判断）
            let input_path = Path::new(&item.input);
            let is_likely_file = input_path.extension().is_some();

            if is_likely_file {
                // 如果 input 看起来是文件，则 output 也应该是文件
                if actual_path.exists() && !actual_path.is_dir() {
                    files_to_remove.push(actual_path);
                }
            } else {
                // 如果 input 是目录，则收集 output 目录下的所有文件
                if actual_path.exists() {
                    let files = utils::collect_files(&actual_path, None)?;
                    for (file, _) in files {
                        if file.exists() && !file.is_dir() {
                            files_to_remove.push(file);
                        }
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

            utils::remove_file_and_folder(file_path).await?;

            let percent = (((index + 1) * 100) / total_files) as u8;
            if let Some(ref callback) = progress {
                callback(percent, &format!("已删除: {} ({}/{})", file_name, index + 1, total_files));
            }
        }
        Ok(())
    }
}
