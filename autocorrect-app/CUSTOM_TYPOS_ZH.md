# 自定义拼写错误检测配置

AutoCorrect 桌面应用内置了 `typos` 库来检测常见的英语拼写错误。但是 `typos` 库有一些限制，它主要检测**常见的拼写错误**，而不是所有可能的拼写错误。

## 为什么 "whts" 没有被检测到？

`typos` 库使用精心策划的拼写错误字典。默认情况下，它**不会检测**：

❌ 不常见的缩写（如 "whts"）
❌ 太短的单词（如 "wat"）  
❌ 上下文相关的错误（如 "there" vs "their"）
❌ 技术术语或专有名词

✅ 但它**会检测**常见的拼写错误：

- `recieve` → `receive`
- `definately` → `definitely`
- `seperate` → `separate`
- `occured` → `occurred`

## 解决方案：添加自定义拼写错误检测

如果你需要检测特定的拼写错误（如 "whts"），可以创建自定义配置文件。

### 步骤 1：创建自定义配置文件

在你的用户目录创建文件：`~/.autocorrect-typos.txt`

```bash
# macOS/Linux
touch ~/.autocorrect-typos.txt

# Windows
type nul > %USERPROFILE%\.autocorrect-typos.txt
```

### 步骤 2：添加自定义拼写错误映射

编辑 `~/.autocorrect-typos.txt` 文件，格式为 `typo=correction`：

```
# 自定义拼写错误修正
whts=what's
teh=the
waht=what
fo=for
recieve=receive
```

### 步骤 3：重启 AutoCorrect 应用

保存文件后，重启 AutoCorrect 应用，自定义配置将自动加载。

### 步骤 4：测试

选择包含 "whts" 的文本，按下全局快捷键（默认 `Cmd+Shift+K`），现在应该能看到：

```
Spelling Issues Found:
- whts → what's
```

## 文件格式说明

`~/.autocorrect-typos.txt` 文件格式：

```
# 以 # 开头的是注释
# 格式：拼写错误=正确拼写

whts=what's
teh=the
waht=what

# 可以添加任意多的映射
recieve=receive
definately=definitely
```

## 测试你的配置

在终端运行测试来验证配置是否生效：

```bash
cd autocorrect-app/src-tauri
cargo test --lib typocheck::tests::test_whts_with_custom -- --nocapture
```

应该看到：

```
Text: 'whts is your name'
Found 1 typos
  - 'whts' -> ["what's"]
✅ Custom correction for 'whts' is working!
```

## 常见问题

### Q: 为什么我的自定义配置没有生效？

A: 确保：

1. 文件路径正确：`~/.autocorrect-typos.txt`
2. 文件格式正确：`typo=correction` （一行一个）
3. 重启了 AutoCorrect 应用
4. 拼写错误检测已启用（Settings → Enable Advanced Typo Detection）

### Q: 我可以完全禁用拼写检测吗？

A: 可以。在 AutoCorrect 设置中：

1. 打开设置面板
2. 找到 "Enable Advanced Typo Detection"
3. 关闭开关
4. 保存更改

### Q: 我想忽略某些被误报的单词怎么办？

A: 使用 AutoCorrect 的自定义字典功能：

1. 打开设置 → Spell Check Configuration
2. 在 "Custom Dictionary" 文本框添加词汇（每行一个）
3. 保存更改

## 技术细节

- **默认检测器**：`typos` 库（检测常见拼写错误）
- **自定义检测器**：读取 `~/.autocorrect-typos.txt` 文件
- **优先级**：自定义配置优先于默认检测
- **性能**：配置文件在应用启动时加载一次（LazyLock）

## 其他选择

如果你需要更全面的拼写检查，可以考虑：

1. **LanguageTool** - 语法和高级拼写检查
2. **Hunspell** - LibreOffice、Firefox 使用
3. **操作系统内置** - macOS/Windows 拼写检查
4. **只使用 AutoCorrect 的 CJK 格式化** - 禁用拼写检测功能

AutoCorrect 专注于 CJK 文本格式化，英语拼写检查是额外的bonus功能。
