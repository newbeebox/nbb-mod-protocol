//! 复制文件命令

use super::super::proto::*;
use super::command_base::{CommandParams, CommandTrait, ProgressCallbackOption};
use super::utils;
use std::path::{Path, PathBuf};

/// 收集所有文件的源路径和目标路径
fn collect_all_files(
    cmd_params: &[FileCopyParam],
    mod_dir: &Path,
    envs: &ModEnvMap,
) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
    let mut all_files = Vec::new();
    for item in cmd_params.iter() {
        let source_path = mod_dir.join(&item.input);
        let target_path = PathBuf::from(utils::env_replace(envs, &item.output)?);

        let files = utils::collect_files(&source_path, Some(mod_dir))?;
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
    Ok(all_files)
}

impl CommandTrait for FileCopyCommand {
    async fn install(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        _all_commands: &[Command],
    ) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        let all_files = collect_all_files(&self.params, &mod_dir, &params.envs)?;
        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制");
            }
            return Ok(());
        }

        for (index, (input, output)) in all_files.iter().enumerate() {
            let file_name = input
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("正在复制: {} ({}/{})", file_name, index + 1, total_files),
                );
            }

            utils::copy_file(input, output).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("已复制: {} ({}/{})", file_name, index + 1, total_files),
                );
            }
        }

        Ok(())
    }

    async fn remove(
        &self,
        params: CommandParams,
        progress: ProgressCallbackOption,
        _all_commands: &[Command],
    ) -> anyhow::Result<()> {
        let mod_dir = params.get_mod_dir()?;

        // 根据命令参数计算目标文件路径
        let all_files = collect_all_files(&self.params, &mod_dir, &params.envs)?;
        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要删除");
            }
            return Ok(());
        }

        for (index, (_input, output)) in all_files.iter().enumerate() {
            let file_name = output
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("正在删除: {} ({}/{})", file_name, index + 1, total_files),
                );
            }

            utils::remove_file_and_folder(output).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(
                    percent,
                    &format!("已删除: {} ({}/{})", file_name, index + 1, total_files),
                );
            }
        }

        Ok(())
    }
}
