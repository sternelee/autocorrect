# Premium Features Roadmap

**Date:** 2026-03-17
**Status:** Draft

## Overview

AutoCorrect App 付费高级功能方向规划。基于当前 Tauri 2 + Svelte 5 架构和已有的 autocorrect 核心 + AI 集成能力，列出可商业化的功能方向。

## 方向一：AI 增强类

### AI 润色/改写

- 扩展当前 `ai_grammar.rs` 的能力，支持多风格改写（正式/口语/学术/商务）
- 按调用量或订阅计费
- 技术基础：已有 OpenAI-compatible streaming API 集成

### AI 翻译

- 选中文本直接翻译，结合 CJK 格式化优势做中英互译
- 复用现有 popup/ai-popup 窗口交互流程

### 上下文感知纠错

- 基于整篇文档语境而非单句纠错，提高准确率
- 需要扩展当前 spellcheck pipeline，增加文档级别的上下文传递

### 自定义 AI Prompt 模板

- 用户自建常用的文本处理流水线
- 存储在 `~/.autocorrect-app.json` 或独立配置文件

## 方向二：专业写作工具

### 术语表/团队词库同步

- 团队共享自定义词典和纠正规则
- 适合企业场景，基于现有 `custom_corrections.rs` 扩展
- 需要云端同步服务

### 多语言风格指南

- 不同项目配置不同排版规范（技术文档 vs 营销文案）
- 扩展 `.autocorrectrc` 支持 profile 切换

### Markdown/LaTeX 深度支持

- 识别文档结构，只纠正正文部分，保留代码和公式
- 核心 autocorrect 已有代码解析能力（28+ 语言），可扩展到文档格式

## 方向三：系统集成类

### 全局实时纠错

- 当前是 hotkey 触发，付费版可做输入法级别的实时检测
- 利用已有的 Accessibility + overlay 架构（`macos_text.rs` + `overlay.rs`）
- 技术挑战：性能和隐私平衡

### 多应用深度集成

- 针对 IDE、邮件客户端、Notion 等做专属适配
- 扩展 `ignored_apps.rs` 为应用级别的定制规则

### 剪贴板历史 + 自动格式化

- 复制即纠正，带历史回溯
- 基于现有 `clipboard.rs` 扩展

## 方向四：团队/企业版

### 云端规则同步

- 跨设备同步 `.autocorrectrc` 和自定义词典
- 需要后端服务

### 使用统计仪表盘

- 纠正频次、常见错误类型分析
- 本地统计可先行，云端聚合为企业版

### 管理员统一配置下发

- 企业 IT 统一推送规则和词库

## 方向五：跨平台扩展

### Windows/Linux 支持

- 当前仅 macOS（依赖 AX API），跨平台是天然付费点
- 需要重写 `macos_text.rs`、`text_selection.rs` 的平台抽象层

### 浏览器插件

- 复用 autocorrect WASM 核心（父项目已有 `make wasm`），网页端实时纠错

### iOS/Android 键盘

- 移动端输入法集成

## 优先级建议

| 优先级 | 功能 | 原因 |
|--------|------|------|
| P0 | AI 润色/改写 | 已有 AI 基础设施，开发量最小 |
| P0 | 团队词库同步 | 企业付费意愿强 |
| P1 | 全局实时纠错 | 已有 AX + overlay 架构基础 |
| P1 | 浏览器插件 | 已有 WASM 核心 |
| P2 | 跨平台 | 工程量大但市场价值高 |
| P2 | 企业管理后台 | 需要后端服务投入 |

## 技术依赖关系

```
AI 润色/改写 ← ai_grammar.rs (已有)
团队词库同步 ← custom_corrections.rs + 云端服务 (新增)
全局实时纠错 ← macos_text.rs + overlay.rs (扩展)
浏览器插件 ← autocorrect WASM (父项目已有)
跨平台 ← 平台抽象层重写 (大工程)
```
