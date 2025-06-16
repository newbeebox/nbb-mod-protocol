use crate::server::command_base::CommandTrait;
use crate::*;

impl CommandTrait for LauncherArgCommand {
    async fn install(&self, _params: CommandParams) -> anyhow::Result<()> {
        Ok(())
    }

    async fn remove(&self, _params: CommandParams) -> anyhow::Result<()> {
        Ok(())
    }
}
