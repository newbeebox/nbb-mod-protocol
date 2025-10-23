# 游戏模组协议 v1.0.0

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

协议中的路径支持以下占位符：

- `{{GameRootDir}}` - 游戏根目录
- `{{HomeDir}}` - 用户主目录

## 方法列表

### launcher_arg - 启动参数

```json
{
  "method": "launcher_arg",
  "params": ["arg1", "arg2"]
}
```

### copy_to_game_dir - 复制到游戏目录

```json
{
  "method": "copy_to_game_dir",
  "params": [
    "Mods/file1.pak",
    "Mods/file2.pak"
  ]
}
```

- 参数：相对于游戏目录的路径列表
- 卸载时自动查找 `rename_mapping.json` 获取重命名后的文件名

### copy_to_home_dir - 复制到用户目录

```json
{
  "method": "copy_to_home_dir",
  "params": [
    "AppData/Local/Game/config.json"
  ]
}
```

- 参数：相对于用户目录的路径列表
- 卸载逻辑同 `copy_to_game_dir`

### rename_sort - 批量重命名排序

按修改时间排序文件并重新编号，自动去重，生成映射表用于卸载。

```json
{
  "method": "rename_sort",
  "params": {
    "dir": "{{GameRootDir}}/Mods",
    "pattern": "*.pak",
    "format": "MyMod_{index}.pak",
    "index_start": 1,
    "index_padding": 2
  }
}
```

**参数**：

- `dir` - 目标目录（支持环境变量）
- `pattern` - glob 模式，如 `*.pak`
- `format` - 格式字符串，必须包含 `{index}` 占位符
- `index_start` - 起始索引（可选，默认 1）
- `index_padding` - 索引位数（可选，默认 2）

**行为**：

1. 扫描匹配文件，按修改时间排序
2. 计算 SHA256 哈希，相同内容的文件只保留最早的，删除重复
3. 原子性重命名（两阶段提交）
4. 生成 `rename_mapping.json` 映射表到目标目录

**映射表结构**：

```json
{
  "original_name.pak": {
    "new_name": "MyMod_01.pak",
    "hash": "sha256..."
  }
}
```

- 重复文件的多个原始名映射到同一个新名
- 卸载时 `copy_*` 命令自动查表，找到实际文件名并删除
- 自动去重，同一文件只删除一次

**示例**：

```json
[
  {
    "method": "copy_to_game_dir",
    "params": ["Mods/a.pak", "Mods/b.pak"]
  },
  {
    "method": "rename_sort",
    "params": {
      "dir": "{{GameRootDir}}/Mods",
      "pattern": "*.pak",
      "format": "MyMod_{index}.pak"
    }
  }
]
```

安装后：`a.pak` → `MyMod_01.pak`，`b.pak` → `MyMod_02.pak`
卸载时：查表得知 `a.pak` 现在叫 `MyMod_01.pak`，正确删除

---

### copy_file - 通用复制 (已废弃)

仅用于兼容旧协议，新项目请使用 `copy_to_game_dir` 或 `copy_to_home_dir`。

```json
{
  "method": "copy_file",
  "params": [
    {
      "input": "file.pak",
      "output": "{{GameRootDir}}/file.pak"
    }
  ]
}
```
