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
        // 收集所有需要删除的文件，直接从协议中的 output 路径删除
        let mut files_to_remove = Vec::new();
        for item in self.params.iter() {
            let target_path = env_replace(&params.envs, &item.output)?;
            let target_path = PathBuf::from(target_path);

            // 判断 input 是文件还是目录（通过是否有扩展名简单判断）
            let input_path = Path::new(&item.input);
            let is_likely_file = input_path.extension().is_some();

            if is_likely_file {
                // 如果 input 看起来是文件，则 output 也应该是文件
                if target_path.exists() && !target_path.is_dir() {
                    files_to_remove.push(target_path);
                }
            } else {
                // 如果 input 是目录，则收集 output 目录下的所有文件
                if target_path.exists() {
                    let files = utils::collect_files(&target_path, None)?;
                    for (file, _) in files {
                        if file.exists() && !file.is_dir() {
                            files_to_remove.push(file);
                        }
                    }
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
