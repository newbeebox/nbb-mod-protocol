//！ 定义游戏模组变量
use std::collections::HashMap;

/// 游戏模组环境变量
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModEnvKey {
    /// 游戏根路径
    GameRootDir,
    /// 系统用户根路径
    HomeDir,
}

impl ModEnvKey {
    pub fn from(key: &str) -> ModEnvKey {
        match key {
            "GameRootDir" => ModEnvKey::GameRootDir,
            "HomeDir" => ModEnvKey::HomeDir,
            _ => panic!("Invalid key"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModEnvMap {
    inner: HashMap<ModEnvKey, String>,
}

impl Default for ModEnvMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ModEnvMap {
    pub fn new() -> Self {
        let mut env = Self {
            inner: HashMap::new(),
        };

        // 系统用户根路径
        let home_dir = dirs::home_dir()
            .unwrap_or_default()
            .display()
            .to_string()
            .replace("\\", "/");
        env.insert(ModEnvKey::HomeDir, home_dir);

        env
    }

    pub fn insert(&mut self, key: ModEnvKey, value: String) -> Option<String> {
        self.inner.insert(key, value)
    }

    pub fn get(&self, key: ModEnvKey) -> Option<&String> {
        self.inner.get(&key)
    }
}
