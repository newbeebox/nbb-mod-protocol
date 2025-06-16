use crate::*;

impl Builder {
    pub fn launcher_arg(&mut self, args: Vec<String>) -> anyhow::Result<()> {
        self.add(Command::LauncherArg(LauncherArgCommand { params: args }));
        Ok(())
    }
}
