use std::io::ErrorKind;
use std::path::Path;
use anyhow::anyhow;
use crate::Command;

/// 构建模组协议
pub struct Builder {
    inner: Vec<Command>,
    file: String,
}

impl Builder {
    pub async fn from(file_path: &str) -> anyhow::Result<Self> {
        match tokio::fs::read_to_string(file_path).await {
            Ok(json) => {
                let cmd = serde_json::from_str(&json)?;
                Ok(Self {
                    inner: cmd,
                    file: file_path.to_owned(),
                })
            }
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self {
                inner: Vec::new(),
                file: file_path.to_owned(),
            }),
            Err(e) => Err(anyhow!(e)),
        }
    }
    pub async fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.inner)?;
        let file_path = self.file.clone();
        if let Some(parent) = Path::new(&file_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(file_path, json).await?;
        Ok(())
    }

    pub fn commands(&self) -> &[Command] {
        &self.inner
    }
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    pub fn add(&mut self, cmd: Command) {
        self.inner.push(cmd)
    }
    pub fn remove(&mut self, cmd: Command) -> anyhow::Result<()> {
        if let Some(index) = self.inner.iter().position(|x| x == &cmd) {
            self.inner.remove(index);
        } else {
            return Err(anyhow::anyhow!("找不到该方法"));
        }
        Ok(())
    }
}
