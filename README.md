# 游戏模组协议

客户端生成协议文件，服务端根据协议执行安装/卸载。

## 协议格式

```json
[
  {
    "method": "<方法名>",
    "params": "<参数>"
  }
]
```

## 环境变量

- `{{GameRootDir}}` - 游戏根目录
- `{{HomeDir}}` - 用户主目录

## 方法

### launcher_arg

游戏启动参数

```json
{
  "method": "launcher_arg",
  "params": ["arg1", "arg2"]
}
```

### copy_to_game_dir

复制文件到游戏目录（相对路径）

```json
{
  "method": "copy_to_game_dir",
  "params": ["Mods/file1.pak", "Mods/file2.pak"]
}
```

### copy_to_home_dir

复制文件到用户目录（相对路径）

```json
{
  "method": "copy_to_home_dir",
  "params": ["AppData/Local/Game/config.json"]
}
```

### copy_file (已废弃)

通用复制，新项目请使用 `copy_to_game_dir` 或 `copy_to_home_dir`

## 文件冲突处理

### 自动重命名
- 目标文件不存在 → 直接复制
- 目标文件存在且内容相同 → 跳过
- 目标文件存在但内容不同 → 自动重命名（`file_0.pak`, `file_1.pak`, ...）

### 模组ID追踪
- 使用协议文件的 SHA256 作为模组ID
- 记录每个文件的安装者到 `.nbmod_mapping`
- 卸载时只删除本模组安装的文件

**限制**：修改协议文件会导致无法卸载之前的安装
