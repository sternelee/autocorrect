# 竞品分析与差距分析（2026-09）

> 基于 Grammarly 2026 年产品形态、主要竞品公开资料，以及本项目代码现状（含 2026-06 评审后已实现的 P0/P1 项）。
> 结论与 6 月评审（`review-2026-06-03.md`）互补：6 月聚焦内部工程，本篇聚焦外部竞争。

## 一、竞争格局速览

| 产品 | 定位 | 与本项目的关系 |
|------|------|----------------|
| **Grammarly** | 全平台写作助手（桌面覆盖层+浏览器扩展+移动键盘），Pro $12/月；Free 含语法/语气/简洁+100 条 AI 提示/月 | 正面对标：系统级下划线+弹窗交互即同形态 |
| **LanguageTool** | 开源规则引擎，30+ 语言（6 门语言有高级规则），$4.99-6.99/月，可自部署，桌面覆盖层应用 | 最现实的技术威胁，也是最佳集成对象（引擎可本地跑） |
| **ProWritingAid** | 长文深度风格分析（20+ 报告：节奏、句式多样性、回声词），作者市场 | 差异化参照，非直接竞争 |
| **QuillBot** | 免费无限语法检查 + 改写/摘要，浏览器优先 | 价格锚点威胁 |
| **WritingTools（开源）/ Charm / Elephas** | macOS 系统级 AI 快捷改写（选中文本→快捷键→改写/语气/摘要） | 与本项目 AI 弹窗模式直接同质竞争 |
| **Apple Writing Tools** | Apple Silicon 内置：任意文本框校对/改写/语气，本地处理+PCC | 平台级威胁：免费、内置、隐私叙事强 |

## 二、本项目已有的差异化优势

1. **CJK 排版规则是全行业独有**：Grammarly / LanguageTool 均不做中日韩混排空格、全半角标点、引号规范（`autocorrect` 核心能力），双语写作场景无竞品。
2. **代码场景支持**（CSpell 编程词典）：Grammarly 在代码上下文全是误报。
3. **本地翻译 + 本地模型**（Apple 框架 + ORT 本地模型）+ 本地拼写引擎：断网可用，隐私叙事成立。
4. Electron 替换链路的工程健壮性（写后验证 + AXValue 重写 + 剪贴板回退 + AX 消息超时）超过多数开源同类。

## 三、核心差距（按优先级）

### P0-1 语法引擎深度

现状：语法检查 = typos 字典 + AI 整段重写，无规则化语法能力（主谓一致/冠词/时态/同音词）。竞品免费档均自带。
**决策**：集成 `languagetool-rust`，本地/离线运行，结构化 issue 输出复用现有渲染管线。

### P0-2 实时性：轮询 → 事件驱动

现状：800ms 自适应轮询（输入时 100ms），首键延迟受空闲间隔限制，轮询线程可能被 AX 阻塞（已有 1.5s 消息超时缓解）。
**决策**：AXObserver 监听焦点元素 `AXValueChanged` / `AXSelectedTextChanged`，通知触发去抖后的即时检查；轮询降级为慢速兜底。

### P1（后续批次）

- **浏览器内体验**：竞品用扩展原生渲染；我们纯 AX 在 Chromium 上脆弱。中期评估 companion 扩展（native messaging）。
- **个人规则/风格指南**：短语级规则（禁用词/替换对/术语表）+ 导入导出；LanguageTool Personal Style Guide 已下探个人档；CJK 品牌用语规范是空白市场。
- **使用统计与反馈闭环**：写作统计仪表盘、AI 结果 👍/👎。

### P2（远期）

- **Windows 版**：`PlatformAccessibility` trait 抽象（AXUIElement / UI Automation / AT-SPI）。
- **Apple Writing Tools**：威胁与机会并存 —— Apple 不做 CJK 排版与第三方应用持续下划线；可将其作为 AI 动作的后端之一（macOS 15+ 可用时走系统 API）。

### P3（不建议跟）

移动键盘、查重（16B 网页库）、文档级 readability 报告 —— 与"系统级 CJK/代码写作质检"定位无关。

## 四、战略定位

**不做"更好的 Grammarly"，做 Grammarly 和 LanguageTool 都做不了的：双语（CJK×EN）写作质检 + 开发者文档场景 + 本地隐私。**

## 参考资料

- [Grammarly Pro](https://www.grammarly.com/pro) · [Grammarly Plagiarism](https://www.grammarly.com/plagiarism-checker)
- [DemandSage – Grammarly Alternatives 2026](https://www.demandsage.com) · [eesel AI – 7 Best Alternatives](https://www.eesel.ai)
- [Kindlepreneur – ProWritingAid vs Grammarly](https://kindlepreneur.com) · [Manuscript Report – LT vs Grammarly](https://manuscriptreport.com)
- [Grammarly Desktop Alternatives](https://chrischinchilla.com) · [WritingTools](https://github.com/theJayTea/WritingTools)
- [Apple Writing Tools](https://support.apple.com/guide/mac-help/writing-tools-write-improve-summarize-mchldcd6c260/mac) · [Apple Intelligence 隐私](https://www.apple.com/legal/privacy/data/en/intelligence-engine)
