# CSpell 功能调试指南

## 问题诊断

你启用了 CSpell 但没有效果，可能的原因和解决方案：

### 1. 应用需要重启

**症状：**

- 在 Settings 中启用了 CSpell
- 保存了配置
- 但拼写检查时没有检测到错误

**解决方案：**

```bash
# 停止当前运行的应用（在终端按 Ctrl+C）
# 然后重新启动：
cd autocorrect-app
pnpm tauri dev
```

### 2. CSpell 路径问题（已修复）

我们刚刚修复了路径查找逻辑，添加了更智能的路径搜索：

- 从可执行文件路径向上查找
- 支持多种目录结构
- 添加了详细的调试日志

### 3. 测试说明

**重要提示：** CSpell 和 typos 库检测的内容不同

| 测试文本   | typos 库  | CSpell    | 说明               |
| ---------- | --------- | --------- | ------------------ |
| "whats"    | ❌ 不检测 | ❌ 不检测 | 可能被认为是合法词 |
| "hows"     | ❌ 不检测 | ❌ 不检测 | 口语形式           |
| "areyour"  | ✅ 检测   | ✅ 检测   | 明显的拼接错误     |
| "funciton" | ✅ 检测   | ✅ 检测   | 常见拼写错误       |
| "naem"     | ✅ 检测   | ✅ 检测   | 字母颠倒           |

**推荐测试文本：**

```
naem is wrong and funciton too
```

这应该能检测到 "naem" 和 "funciton"。

## 完整测试步骤

### Step 1: 确认配置文件

```bash
cat ~/.autocorrect-app.json
```

应该看到：

```json
{
  "cspell_enabled": true,
  "cspell_dictionaries": {
    "typescript": true,
    ...
  }
}
```

### Step 2: 停止并重启应用

```bash
# 在运行 pnpm tauri dev 的终端按 Ctrl+C
# 然后重新运行：
cd /Users/sternelee/www/github/autocorrect/autocorrect-app
pnpm tauri dev
```

### Step 3: 启用调试日志（可选）

```bash
cd autocorrect-app
RUST_LOG=debug pnpm tauri dev
```

这会在终端显示详细日志，包括：

- CSpell 路径查找过程
- 启用的词典列表
- 检测到的错误数量

### Step 4: 测试

1. 在任意应用中输入：`naem is wrong and funciton too`
2. 选中文本
3. 按 Cmd+Shift+K（或你配置的热键）
4. 应该看到弹出窗口显示 2 个错误：
   - "naem" → suggestions: ["name"]
   - "funciton" → suggestions: ["function"]

## 验证 CSpell 是否工作

### 命令行测试

```bash
cd autocorrect-app

# 测试 1: CSpell CLI 直接测试
echo "naem is wrong and funciton too" | \
  pnpm cspell stdin --no-progress --reporter @cspell/cspell-json-reporter

# 应该输出包含 "naem" 和 "funciton" 的 JSON

# 测试 2: Rust 单元测试
cd src-tauri
cargo test --lib cspell::tests::test_cspell_hows_areyour -- --nocapture

# 应该看到：
# === Testing: 'hows areyour' ===
# Found 1 errors:
#   - 'areyour' at line 1 col 6
```

## 常见问题

### Q1: "whats" 和 "hows" 为什么不被检测？

**答：** 这些词在某些英语词典中被认为是合法的（口语/非正式用法）。

**解决方案：** 使用 Custom Corrections 功能：

1. 打开 Settings → Custom Typo Corrections
2. 添加：
   - `whats=what's`
   - `hows=how's`
3. 保存
4. 再次测试，应该能检测到了

### Q2: 编程术语被误报为错误

**答：** 确保启用了相关的 CSpell 词典。

**解决方案：**

1. Settings → Spell Check Configuration
2. 启用 "Enable CSpell Programming Dictionaries"
3. 选择你使用的语言：
   - TypeScript (对于 React, useState 等)
   - Python (对于 def, import 等)
   - Node (对于 require, module 等)
4. 保存并重启应用

### Q3: 日志显示 "CSpell not found"

**答：** 路径查找失败。

**解决方案：**

```bash
# 确认 CSpell 已安装
cd autocorrect-app
ls -la node_modules/.bin/cspell

# 如果不存在，重新安装：
pnpm install

# 确认安装成功：
pnpm cspell --version
```

### Q4: 性能很慢

**答：** CSpell 需要启动subprocess，第一次检查会慢一些（~200ms）。

**优化建议：**

- 只启用你实际使用的词典
- 对于纯文本，可以禁用 CSpell，只用 typos 库
- 对于代码，两个都启用效果最好

## 工作原理

```
用户触发拼写检查（Cmd+Shift+K）
    ↓
spell_check() 函数被调用
    ↓
1. AutoCorrect CJK 格式化
2. typos 库检查（如果启用）
   - 快速检查常见英语错误
   - 检查自定义修正（~/.autocorrect-typos.txt）
3. CSpell 检查（如果启用）
   - 启动 cspell subprocess
   - 传递选中的词典
   - 解析 JSON 输出
4. 合并并去重结果
    ↓
显示在弹出窗口
```

## 推荐配置

### 对于 Web 开发者

```
✅ typo_checking_enabled: true
✅ cspell_enabled: true
✅ 启用词典：
   - TypeScript
   - HTML
   - CSS
   - Node.js
   - NPM
   - React (如果使用)
   - Git
   - Companies
   - Software Terms
```

### 对于纯文本写作

```
✅ typo_checking_enabled: true
❌ cspell_enabled: false
✅ 添加自定义修正：
   - whats=what's
   - hows=how's
   - teh=the
```

## 下一步

如果按照上述步骤操作后仍有问题：

1. **查看日志：**

   ```bash
   cd autocorrect-app
   RUST_LOG=debug pnpm tauri dev 2>&1 | grep -i cspell
   ```

2. **检查进程：**

   ```bash
   ps aux | grep cspell
   ```

3. **完全重新编译：**

   ```bash
   cd autocorrect-app/src-tauri
   cargo clean
   cd ..
   pnpm tauri build
   ```

4. **提供反馈：**
   - 使用的测试文本
   - 终端日志输出
   - `~/.autocorrect-app.json` 内容
   - 是否重启了应用

---

**快速诊断命令：**

```bash
# 一键测试所有组件
cd /Users/sternelee/www/github/autocorrect/autocorrect-app

echo "1. Checking CSpell CLI..."
pnpm cspell --version

echo "2. Testing CSpell with sample text..."
echo "naem funciton" | pnpm cspell stdin --no-progress

echo "3. Checking config file..."
cat ~/.autocorrect-app.json | grep -A 5 cspell

echo "4. Running Rust tests..."
cd src-tauri && cargo test --lib cspell::tests --quiet

echo "✅ All checks complete!"
```
