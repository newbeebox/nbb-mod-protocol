use crate::server::command_base::{CommandTrait, ProgressCallbackOption};
use crate::server::utils;
use crate::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl CommandTrait for FileCopyToHomeDirCommand {
    async fn install(&self, params: CommandParams, progress: ProgressCallbackOption, _all_commands: &[Command]) -> anyhow::Result<()> {
        let home_dir = params.get_home_dir()?;
        let mod_dir = params.get_mod_dir()?;
        let mod_id = params.get_mod_id().await?;

        let all_files = utils::collect_files_with_mapping(&self.params, &mod_dir, &home_dir)?;
        let total_files = all_files.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要复制到用户目录");
            }
            return Ok(());
        }

        let mut dir_mappings = HashMap::new();

        for (index, (input, output)) in all_files.iter().enumerate() {
            let file_name = input.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(percent, &format!("正在复制到用户目录: {} ({}/{})", file_name, index + 1, total_files));
            }

            utils::copy_and_record_mapping(input, output, &mod_id, &mut dir_mappings).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(percent, &format!("已复制到用户目录: {} ({}/{})", file_name, index + 1, total_files));
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
        let home_dir = params.get_home_dir()?;
        let mod_id = params.get_mod_id().await?;

        let path_infos: Vec<_> = self.params
            .iter()
            .map(|path_str| {
                let path = Path::new(path_str);
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(path_str);
                let parent = path.parent().unwrap_or(Path::new(""));
                let target_dir = home_dir.join(parent);
                (file_name, target_dir)
            })
            .collect();

        let files_to_remove = utils::process_removal_mappings(&path_infos, &mod_id).await?;

        let total_files = files_to_remove.len();
        if total_files == 0 {
            if let Some(ref callback) = progress {
                callback(100, "没有文件需要从用户目录删除");
            }
            return Ok(());
        }

        for (index, file_path) in files_to_remove.iter().enumerate() {
            let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("未知文件");

            if let Some(ref callback) = progress {
                let percent = ((index * 100) / total_files) as u8;
                callback(percent, &format!("正在从用户目录删除: {} ({}/{})", file_name, index + 1, total_files));
            }

            utils::remove_file_and_folder(file_path).await?;

            if let Some(ref callback) = progress {
                let percent = (((index + 1) * 100) / total_files) as u8;
                callback(percent, &format!("已从用户目录删除: {} ({}/{})", file_name, index + 1, total_files));
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
