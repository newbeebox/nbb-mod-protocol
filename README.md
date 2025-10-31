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
- 卸载时自动查找 `.rename_mapping` 获取重命名后的文件名

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

### rename_sort - 智能重命名排序

扫描目录中的文件，按**前缀分组**，智能累加索引避免冲突，自动去重。

```json
{
  "method": "rename_sort",
  "params": {
    "dir": "{{GameRootDir}}/Mods"
  }
}
```

**参数**：

- `dir` - 目标目录（支持环境变量）

**行为**：

1. 扫描目录中的所有文件（仅第一层，不递归）
2. **只处理包含 `_数字` 的文件**，不符合规范的文件保持原样
3. 解析文件名格式：`prefix_index` 或 `prefix_index.suffix`
   - 示例：`9ba626afa44a3aa3.patch_0` → 前缀=`9ba626afa44a3aa3.patch`，索引=`0`
   - 示例：`9ba626afa44a3aa3.patch_0.stream` → 前缀=`9ba626afa44a3aa3.patch`，索引=`0`，后缀=`.stream`
4. 按前缀分组，每组独立处理：
   - 按**修改时间**排序（旧到新）
   - 计算 SHA256 哈希，去重（相同内容只保留最早的文件）
   - 重新编号为 `prefix_0`, `prefix_1`, `prefix_2`...
5. 生成 `.rename_mapping` 映射表到目标目录
6. 卸载时查找映射表删除文件，然后重新排序

**命名规范**：

| 原文件名 | 新文件名 | 说明 |
|---------|---------|------|
| `9ba626afa44a3aa3.patch_0` | `9ba626afa44a3aa3.patch_0` | 符合规范，前缀=`9ba626afa44a3aa3.patch`，索引=0 |
| `9ba626afa44a3aa3.patch_1` | `9ba626afa44a3aa3.patch_1` | 前缀相同，索引=1 |
| `9ba626afa44a3aa3.patch_0.stream` | `9ba626afa44a3aa3.patch_0.stream` | 带后缀 `.stream` |
| `myfile.pak` | `myfile.pak` | ❌ 不符合规范（不包含 `_数字`），保持原样 |

**映射表结构**：

```json
{
  "9ba626afa44a3aa3.patch_0": {
    "new_name": "9ba626afa44a3aa3.patch_0",
    "hash": "sha256..."
  }
}
```

**示例**：

```json
[
  {
    "method": "copy_to_game_dir",
    "params": ["Mods/mod_a.pak", "Mods/mod_b.pak"]
  },
  {
    "method": "rename_sort",
    "params": {
      "dir": "{{GameRootDir}}/Mods"
    }
  }
]
```

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
