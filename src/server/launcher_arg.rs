use crate::server::command_base::{CommandTrait, ProgressCallbackOption};
use crate::*;

impl CommandTrait for LauncherArgCommand {
    async fn install(&self, _params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        if let Some(callback) = progress {
            callback(50, "设置启动参数");
            callback(100, "启动参数设置完成");
        }
        Ok(())
    }

    async fn remove(&self, _params: CommandParams, progress: ProgressCallbackOption) -> anyhow::Result<()> {
        if let Some(callback) = progress {
            callback(50, "清除启动参数");
            callback(100, "启动参数已清除");
        }
        Ok(())
    }
}
