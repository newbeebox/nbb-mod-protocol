//! 复制文件命令

use super::super::proto::*;
use super::command_base::{CommandTrait, CommandParams, ProgressCallbackOption};
use super::utils;
use anyhow::anyhow;
use regex::Regex;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 替换环境变量
fn env_replace(env: &ModEnvMap, input: &str) -> anyhow::Result<String> {
    let regex = Regex::new(r"\{\{(\w+)}}")?;

    let path = regex
        .replace_all(input, |caps: &regex::Captures| {
            let key = &caps[1];
            env.get(ModEnvKey::from(key))
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();
    Ok(path)
}

impl CommandTrait for FileCopyCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        // 收集所有需要复制的文件
        let mut all_files = Vec::new();
        for item in self.params.iter() {
            let source_path = mod_dir.join(&item.input);
            let target_path = env_replace(&params.envs, &item.output)?;
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

    async fn remove(&self, params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        // 收集所有可能需要删除的文件
        let mut files_to_remove = Vec::new();
        for item in self.params.iter() {
            let source_path = mod_dir.join(&item.input);
            let target_path = env_replace(&params.envs, &item.output)?;
            let target_path = PathBuf::from(target_path);

            // 使用工具函数收集源文件列表，然后检查对应的目标文件
            let files = utils::collect_files(&source_path, Some(&mod_dir))?;
            for (source, _relative) in files {
                // 计算对应的目标文件路径
                let target = if let Ok(rel) = source.strip_prefix(&source_path) {
                    if rel.as_os_str().is_empty() {
                        target_path.clone()
                    } else {
                        target_path.join(rel)
                    }
                } else {
                    target_path.clone()
                };

                // 只删除实际存在的文件
                if target.exists() && !target.is_dir() {
                    files_to_remove.push(target);
                }
            }
        }

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
