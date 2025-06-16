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

    #[tokio::test]
    async fn test_builder() {
        let mut builder = Builder::from("g:\\12201-32\\install.json").await.unwrap();
        builder.copy_to_game_dir("g:/12201-32").unwrap();
        builder.launcher_arg(vec!["test".to_owned()]).unwrap();
        builder.save().await.unwrap();
    }

    #[tokio::test]
    async fn test_install() {
        let dto = CommandParams::from("g:/games/test", Some("g:/12201-32".to_owned()));
        install(dto).await.unwrap();
    }

    #[tokio::test]
    async fn test_remove() {
        let dto = CommandParams::from("g:/games/test", None);
        remove("g:\\12201-32\\install.json", dto).await.unwrap();
    }
}
