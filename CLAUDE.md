# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个游戏模组协议处理库，使用 Rust 编写。该库定义了客户端生成协议文件、服务端根据协议执行相应方法的标准化流程。

## 核心架构

### 模块结构
- `src/proto/` - 协议定义层，包含所有协议命令的数据结构
- `src/client/` - 客户端实现，用于构建协议文件
- `src/server/` - 服务端实现，执行协议命令（安装/卸载模组）
- `src/lib.rs` - 库入口，导出所有公共接口

### 核心概念
1. **Command枚举** - 所有协议命令的统一抽象，支持的方法包括：
   - `launcher_arg` - 游戏启动参数
   - `copy_to_game_dir` - 复制文件到游戏目录
   - `copy_to_home_dir` - 复制文件到用户主目录
   - `copy_file` - (已过时) 通用文件复制

2. **Builder模式** - 客户端使用Builder构建协议文件，支持链式调用和持久化

3. **执行器模式** - 服务端通过统一的executor执行不同类型的命令

## 开发命令

### 构建项目
```bash
cargo build
```

### 运行测试
```bash
cargo test
```

### 运行特定测试
```bash
cargo test test_builder
cargo test test_install
cargo test test_remove
```

### 发布构建
```bash
cargo build --release
```

### 检查代码
```bash
cargo check
cargo clippy
```

## 协议文件格式

协议文件为JSON格式，包含一个命令数组：
```json
[
  {
    "method": "<方法名>",
    "params": "<请求参数>"
  }
]
```

## 关键数据流

1. **客户端流程**：Builder从文件加载/创建新协议 → 添加命令 → 保存到JSON文件
2. **服务端流程**：读取协议文件 → 解析命令列表 → 依次执行命令 → 返回结果

## 环境变量替换

协议支持以下环境变量占位符：
- `{{GameRootDir}}` - 游戏根目录
- `{{HomeDir}}` - 用户主目录

这些占位符在服务端执行时会被实际路径替换。