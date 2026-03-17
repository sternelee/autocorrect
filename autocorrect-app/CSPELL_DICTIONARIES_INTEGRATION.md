# CSpell 默认集成词典文档

## 概述

CSpell 是一个为代码设计的拼写检查器,默认集成了大量的专业词典,无需额外配置即可支持多种编程语言和技术栈。

## 默认词典列表

CSpell 默认集成以下词典 (来源: `cspell-bundled-dicts` 包):

### 🌐 通用词典

- **en_us** - 美式英语词典
- **en-gb-mit** - 英式英语词典
- **en-common-misspellings** - 常见拼写错误词典

### 💼 公司与商业

- **companies** - 知名公司名称 (默认启用)
- **softwareTerms** - 软件术语 (默认启用)
- **public-licenses** - 公共许可证名称 (默认启用)
- **filetypes** - 文件类型扩展名 (默认启用)

### 💻 编程语言

- **ada** - Ada 语言
- **al** - AL (Business Central) 语言
- **bash** - Bash 脚本
- **cpp** - C++ 语言
- **csharp** - C# 语言
- **dart** - Dart 语言
- **dotnet** - .NET 框架
- **elixir** - Elixir 语言
- **fsharp** - F# 语言
- **golang** - Go 语言
- **haskell** - Haskell 语言
- **java** - Java 语言
- **julia** - Julia 语言
- **kotlin** - Kotlin 语言
- **lua** - Lua 语言
- **monkeyc** - MonkeyC 语言
- **php** - PHP 语言
- **powershell** - PowerShell
- **python** - Python 语言
- **r** - R 语言
- **ruby** - Ruby 语言
- **rust** - Rust 语言
- **scala** - Scala 语言
- **shell** - Shell 脚本
- **sql** - SQL 语言
- **swift** - Swift 语言
- **typescript** - TypeScript 语言

### 🎨 前端开发

- **css** - CSS 样式表
- **fonts** - 字体名称
- **html** - HTML 标记
- **html-symbol-entities** - HTML 符号实体 (如 `&clubs;`)
- **markdown** - Markdown 语法
- **svelte** - Svelte 框架
- **vue** - Vue.js 框架

### 🔧 框架与工具

- **aws** - AWS 云服务术语
- **django** - Django 框架
- **docker** - Docker 容器
- **flutter** - Flutter 框架
- **fullstack** - 全栈开发术语
- **git** - Git 版本控制
- **google** - Google 服务术语
- **k8s** - Kubernetes 术语
- **makefile** - Makefile 语法
- **node** - Node.js API
- **npm** - npm 包名称
- **terraform** - Terraform 术语

### 📊 专业领域

- **cryptocurrencies** - 加密货币术语
- **data-science** - 数据科学术语
- **gaming-terms** - 游戏术语
- **latex** - LaTeX 排版
- **lorem-ipsum** - Lorem Ipsum 占位文本

## 语言特定配置

CSpell 根据文件类型自动应用相应的词典：

### JavaScript / TypeScript

```json
{
  "languageId": "javascript,typescript",
  "dictionaries": ["typescript", "node", "npm"]
}
```

### React (JSX/TSX)

```json
{
  "languageId": "javascriptreact,typescriptreact",
  "dictionaries": [
    "typescript",
    "node",
    "npm",
    "html",
    "html-symbol-entities",
    "css",
    "fonts"
  ]
}
```

### HTML / PHP / Handlebars

```json
{
  "languageId": "html,php,handlebars",
  "dictionaries": [
    "html",
    "fonts",
    "typescript",
    "css",
    "npm",
    "html-symbol-entities"
  ]
}
```

### CSS / SCSS / LESS

```json
{
  "languageId": "css,less,scss",
  "dictionaries": ["fonts", "css"]
}
```

### Markdown

```json
{
  "languageId": "markdown",
  "dictionaries": ["npm", "html", "html-symbol-entities"]
}
```

### JSON

```json
{
  "languageId": "json,jsonc",
  "dictionaries": ["node", "npm"]
}
```

## 默认配置

CSpell 的默认配置 (`cspell-default.config.ts`):

```typescript
{
  language: 'en',
  maxNumberOfProblems: 10_000,
  allowCompoundWords: false,

  // 默认启用的词典
  dictionaries: [
    'companies',
    'softwareTerms',
    'public-licenses',
    'filetypes'
  ],

  // 导入所有专业词典
  import: [
    '@cspell/dict-en_us/cspell-ext.json',
    '@cspell/dict-typescript/cspell-ext.json',
    '@cspell/dict-python/cspell-ext.json',
    // ... 50+ 其他词典
  ]
}
```

## 特殊模式识别

CSpell 内置以下模式识别,自动忽略特定内容:

### HTML 符号实体

- 模式: `&[a-z]+;`
- 示例: `&clubs;`, `&hearts;`, `&nbsp;`

### Markdown 链接

- **引用链接**: `[link][reference]` (忽略 `[reference]`)
- **链接脚注**: `[reference]: https://...` (完全忽略)
- **内联链接**: `[text](url)` (忽略 `url`)
- **锚点**: `<a id="my_link"></a>` (忽略 `my_link`)

## 在 AutoCorrect 项目中的应用建议

### 方案 1: 使用 CSpell 作为词典源

可以将 CSpell 的词典导入到 AutoCorrect 中:

```rust
// 在 typocheck.rs 中集成 cspell 词典
use std::process::Command;

fn load_cspell_dictionaries() -> Vec<String> {
    let output = Command::new("cspell")
        .arg("dictionaries")
        .arg("--show-path")
        .output()
        .expect("Failed to run cspell");

    // 解析并加载词典文件
    // ...
}
```

### 方案 2: 作为 Typo 检测的补充

```rust
// 结合 typos + cspell 的双重检测
pub fn check_typos_with_cspell(text: &str) -> Vec<TypoError> {
    let mut errors = Vec::new();

    // 1. 使用 typos 库进行基础检测
    let typos_results = typos::check_str(text, &POLICY.tokenizer, POLICY.dict);

    // 2. 使用 cspell 进行代码专业术语检测
    let cspell_results = run_cspell_check(text);

    // 3. 合并结果
    errors.extend(typos_results);
    errors.extend(cspell_results);

    errors
}
```

### 方案 3: 配置文件集成

在 AutoCorrect 配置中引用 cspell 词典:

```yaml
# ~/.autocorrectrc
typo_checking:
  enabled: true
  sources:
    - type: "built-in" # typos 库
    - type: "cspell" # cspell 词典
      dictionaries:
        - typescript
        - node
        - npm
        - html
        - css
    - type: "custom" # 自定义词典
      path: "~/.autocorrect-typos.txt"
```

## CSpell 词典管理命令

### 查看所有可用词典

```bash
cspell dictionaries
```

### 查看已启用的词典

```bash
cspell dictionaries --enabled
```

### 查看词典路径

```bash
cspell dictionaries --path-format absolute
```

### 测试单词是否在词典中

```bash
cspell trace "typescript"
```

## 优势分析

### CSpell 的优势

1. **专业性强** - 包含 50+ 编程语言和技术栈词典
2. **维护活跃** - GitHub 1.6k stars, 26.3k 项目使用
3. **生态完整** - VS Code 扩展, CLI, ESLint 插件
4. **配置灵活** - 支持 JSON, YAML, JS/TS 配置文件
5. **社区支持** - 可贡献词典到 [cspell-dicts](https://github.com/streetsidesoftware/cspell-dicts)

### 与 Typos 库的对比

| 特性         | CSpell       | Typos            |
| ------------ | ------------ | ---------------- |
| **主要用途** | 代码拼写检查 | 通用英语拼写检查 |
| **词典数量** | 50+ 专业词典 | 1 个通用词典     |
| **技术术语** | ✅ 优秀      | ❌ 较弱          |
| **性能**     | 中等         | 快速             |
| **编程语言** | 50+          | 无特定支持       |
| **自定义**   | 非常灵活     | 基础支持         |

## 推荐集成策略

### 阶段 1: 研究和测试 (当前)

1. ✅ 调研 CSpell 词典格式
2. ✅ 了解默认集成词典
3. ⏳ 评估集成成本

### 阶段 2: 原型开发

1. 创建 Rust FFI 绑定调用 cspell CLI
2. 或者: 解析 cspell 词典文件格式 (trie 格式)
3. 实现词典加载和查询机制

### 阶段 3: UI 集成

1. 在 Settings 中添加 "CSpell 词典" 选项
2. 允许用户选择启用哪些专业词典
3. 显示词典加载状态

### 阶段 4: 性能优化

1. 缓存常用词典
2. 按需加载大型词典
3. 内存优化 (cspell 使用 trie 数据结构)

## 相关资源

- **官网**: https://cspell.org
- **GitHub**: https://github.com/streetsidesoftware/cspell
- **词典仓库**: https://github.com/streetsidesoftware/cspell-dicts
- **VS Code 扩展**: https://marketplace.visualstudio.com/items?itemName=streetsidesoftware.code-spell-checker
- **文档**: https://cspell.org/docs/getting-started

## 代码示例: 读取 CSpell 词典

```rust
// 示例: 解析 cspell 词典文件
use std::fs;
use serde_json::Value;

pub fn load_cspell_dict(dict_name: &str) -> Result<HashSet<String>, String> {
    // CSpell 词典路径
    let dict_path = format!("node_modules/@cspell/dict-{}/cspell-ext.json", dict_name);

    // 读取配置文件
    let content = fs::read_to_string(&dict_path)
        .map_err(|e| format!("Failed to read dictionary: {}", e))?;

    let config: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse dictionary config: {}", e))?;

    // 提取词典文件路径
    let dict_file = config["dictionaryDefinitions"][0]["path"]
        .as_str()
        .ok_or("Dictionary path not found")?;

    // 加载词典内容
    let words_content = fs::read_to_string(dict_file)
        .map_err(|e| format!("Failed to read words file: {}", e))?;

    // 解析单词列表
    let words: HashSet<String> = words_content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.trim().to_lowercase())
        .collect();

    Ok(words)
}
```

## 总结

CSpell 提供了一个成熟的、专为代码设计的拼写检查解决方案,其 50+ 专业词典可以显著提升 AutoCorrect 在技术术语检测方面的能力。建议:

1. **短期**: 继续使用 `typos` 库处理通用英语拼写
2. **中期**: 通过 CLI 集成 CSpell 进行代码术语检测
3. **长期**: 考虑直接解析 CSpell 词典格式,实现原生支持

这样可以实现:

- ✅ 通用拼写: typos 库 (快速)
- ✅ 代码术语: CSpell 词典 (专业)
- ✅ 自定义: ~/.autocorrect-typos.txt (灵活)

三者结合,打造最强大的代码拼写检查工具! 🚀
