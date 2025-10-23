//! 游戏模组安装|卸载协议
#![allow(unused)]
#![allow(deprecated)]

mod client;
mod proto;
mod server;

pub use client::*;
pub use proto::*;
pub use server::*;

#[cfg(test)]
mod test {
    use super::*;
    use std::io::{self, Write};

    const MOD_DIR: &str = r#"G:\NewBeeBoxCache\NewBeeBoxCache\nbCoreDownload\test"#;  

    // cargo test test_install -- --nocapture
    #[tokio::test]
    async fn test_install() {
        use std::sync::Arc;
        let dto = CommandParams::from("g:/games/test", Some(MOD_DIR.to_owned()));
        let progress_callback = Arc::new(|progress: u8, message: &str| {
            eprintln!("进度: {}% - {}", progress, message);
        });
        install(dto, Some(progress_callback)).await.unwrap();
    }

    // cargo test test_remove -- --nocapture
    #[tokio::test]
    async fn test_remove() {
        use std::sync::Arc;
        let dto = CommandParams::from("g:/games/test", None);
        let config = format!("{}\\install.json", MOD_DIR);
        let progress_callback = Arc::new(|progress: u8, message: &str| {
            eprintln!("卸载进度: {}% - {}", progress, message);
        });
        remove(&config, dto, Some(progress_callback)).await.unwrap();
    }
}