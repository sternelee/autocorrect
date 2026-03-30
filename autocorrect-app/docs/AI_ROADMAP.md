# AI 功能路线图

基于 Grammarly 功能对比分析，规划 autocorrect-app 的 AI 优化方向。

## 已实现功能 ✅

| 功能     | 描述              | 实现位置                                   |
| -------- | ----------------- | ------------------------------------------ |
| 语法检查 | AI 驱动的语法纠错 | `ai_text_transform(operation="grammar")`   |
| 文本润色 | 4 种风格润色      | `ai_polish_batch`                          |
| 翻译     | 多语言翻译        | `ai_text_transform(operation="translate")` |
| 流式输出 | 实时流式响应      | `ai_text_transform_stream`                 |

## 待实现功能路线图

### Phase 1: 核心增强 (当前优先)

#### 1. 语气检测与调整

- **语气检测**: 分析文本当前语气 (正式/随意/友好/严肃等)
- **语气调整**: 一键调整至目标语气
- **语气类型**:
  - 正式
  - 随意
  - 友好
  - 自信
  - 严肃
  - 乐观
  - 鼓励
- **实现方案**:
  - 新增 `ai_tone_detect` 命令
  - 新增 `ai_tone_adjust` 命令
  - 返回语气评分和置信度

#### 2. 清晰度与简洁性

- **冗余检测**: 识别可删除的冗余词汇
- **句子复杂度**: 标记过长/复杂的句子
- **可读性评分**: Flesch-Kincaid 等评分体系
- **简洁建议**: 提供更简洁的表达方式
- **实现方案**:
  - 新增 `ai_clarity_check` 命令
  - 返回问题位置和修改建议
  - 提供可读性分数

#### 3. 词汇增强

- **同义词建议**: 为重复或平淡词汇提供替代
- **上下文词汇**: 根据语境推荐更精准的词汇
- **词汇多样性**: 分析词汇使用多样性
- **实现方案**:
  - 新增 `ai_vocabulary_enhance` 命令
  - 返回词汇建议列表及位置

### Phase 2: 高级功能

#### 4. 全句重写

- 一键重写整段
- 扩展/缩短文本
- 保持原意的多种改写版本

#### 5. 抄袭检测

- 网页查重
- 原创性评分
- 来源标注

#### 6. 引用生成

- APA/MLA/Chicago 格式
- 自动格式化引用

### Phase 3: 差异化竞争力

#### 7. CJK 专属语法

- 中文语法深度检查
- 日语敬语/文体检查
- 中日韩混合文本处理

#### 8. AI 内容检测

- 检测 AI 生成内容
- AI 使用披露建议

#### 9. 实时上下文感知

- 根据写作场景调整建议
- 专业术语库
- 自定义词典

#### 10. 批量/多风格对比增强

- 版本差异高亮
- 并排对比视图

## Grammarly 功能参考

### 语气类型 (16+)

Confident, Formal, Informal, Optimistic, Worried, Friendly, Assertive,
Encouraging, Curious, Surprised, Disapproving, Accusatory, Sad, Joyful,
Regretful, Appreciative

### 可读性指标

- Flesch Reading Ease
- Flesch-Kincaid Grade Level
- Gunning Fog Index
- SMOG Index

### 清晰度检查项

- 被动语态检测
- 冗余短语
- 复杂句结构
- 模糊表达

## 技术实现

### API 设计原则

- 保持与现有 `ai_text_transform` 一致的接口风格
- 支持流式和非流式两种模式
- 统一错误处理和超时机制

### 响应格式

```typescript
interface ToneDetectResponse {
  tones: Array<{
    name: string; // 语气名称
    score: number; // 置信度 0-100
    evidence: string[]; // 支持该判断的文本片段
  }>;
  overall: string; // 主要语气
}

interface ClarityCheckResponse {
  score: number; // 可读性分数 0-100
  issues: Array<{
    type: "redundancy" | "complexity" | "passive" | "vague";
    text: string;
    suggestion: string;
    line: number;
    col: number;
  }>;
  stats: {
    avgSentenceLength: number;
    passiveVoiceCount: number;
    readabilityGrade: string;
  };
}

interface VocabularyEnhanceResponse {
  suggestions: Array<{
    original: string;
    line: number;
    col: number;
    alternatives: Array<{
      word: string;
      reason: string; // "more precise" | "more formal" | "less repetitive"
    }>;
  }>;
}
```

## 进度追踪

- [ ] Phase 1.1: 语气检测与调整
- [ ] Phase 1.2: 清晰度与简洁性
- [ ] Phase 1.3: 词汇增强
- [ ] Phase 2: 高级功能
- [ ] Phase 3: 差异化竞争力

## 英文测试语句（回归测试用）

以下语句可直接粘贴到 AI 弹窗，用于验证语气检测、清晰度分析、词汇增强。

### 1) Tone Detection

- `I truly appreciate your quick response, and I’m excited to collaborate on this project.`
- `Please review the attached proposal and provide your feedback by Friday.`
- `Hey team, just a quick heads-up: the release might slip by one day.`
- `The observed variance suggests that the current model is underfitting the dataset.`
- `I strongly disagree with this approach because it introduces unnecessary risk.`

### 2) Clarity / Conciseness

- `In order to be able to successfully complete the task, we need to first start by making an initial plan.`
- `The report was written by the team and was reviewed by the manager before it was submitted.`
- `It is important to note that this feature is basically kind of somewhat useful in many different ways.`
- `The solution, which was developed after several rounds of discussion and multiple iterations with stakeholders from different departments, can potentially improve performance.`
- `We should maybe possibly consider adjusting the timeline a little bit.`

### 3) Vocabulary Enhancement

- `The product is very good, and the user experience is very good as well.`
- `This is a big improvement and a big opportunity for the team.`
- `The results are nice, but we need a better way to present them.`
- `Our strategy is good, but the implementation quality is not good enough.`
- `The app works fine, but the documentation is fine too.`

### 4) Mixed Cases (综合压测)

- `Hi everyone, I wanted to kindly remind you that the draft is still pending, and I would really appreciate it if we could finalize it soon.`
- `This proposal is good, but it could be better if we simplify the structure, remove repetitive wording, and use more precise terminology.`
- `A decision was made by the committee, and it was communicated to the broader organization last week.`

## 预期结果示例（人工验收参考）

说明：以下为“应当出现的方向性结果”，不是唯一答案。不同模型版本可能给出不同措辞。

### A. Tone Detection 预期

1. `I truly appreciate your quick response, and I’m excited to collaborate on this project.`
   - 预期主语气：`friendly` 或 `professional`
   - 次要语气可能包含：`confident`

2. `Please review the attached proposal and provide your feedback by Friday.`
   - 预期主语气：`formal` 或 `business`
   - 分数应明显高于 `informal`

3. `Hey team, just a quick heads-up: the release might slip by one day.`
   - 预期主语气：`informal` 或 `friendly`
   - 不应被判为 `academic`

4. `The observed variance suggests that the current model is underfitting the dataset.`
   - 预期主语气：`academic` 或 `professional`
   - 应体现技术语境

5. `I strongly disagree with this approach because it introduces unnecessary risk.`
   - 预期主语气：`serious` 或 `professional`
   - 情绪应偏坚定，不应偏 `friendly`

### B. Clarity / Conciseness 预期

1. `In order to be able to successfully complete the task, we need to first start by making an initial plan.`
   - 预期命中：`redundancy`（如 `In order to be able to`, `first start`, `initial`）
   - 建议应更短、更直接

2. `The report was written by the team and was reviewed by the manager before it was submitted.`
   - 预期命中：`passive`
   - 建议可能改为主动语态

3. `It is important to note that this feature is basically kind of somewhat useful in many different ways.`
   - 预期命中：`vague` + `redundancy`
   - 建议应去掉模糊副词（`basically`, `kind of`, `somewhat`）

4. 超长复合句样例（stakeholders 那句）
   - 预期命中：`complexity`
   - 建议可能拆句或减少从句层级

5. `We should maybe possibly consider adjusting the timeline a little bit.`
   - 预期命中：`vague` / `redundancy`
   - 建议应减少 hedging（`maybe`, `possibly`, `a little bit`）

### C. Vocabulary Enhancement 预期

1. `very good` / `good` / `nice` / `fine` 高频重复句
   - 预期给出更精准替换（如 `excellent`, `effective`, `clear`, `robust` 等）
   - `reason` 应反映 `more precise` 或 `less repetitive`

2. `big improvement and a big opportunity`
   - 预期至少对一个 `big` 给出替换（如 `significant`, `major`）

3. `Our strategy is good, but the implementation quality is not good enough.`
   - 预期同时对两个 `good` 给出不同上下文建议
   - 不应只给完全同义替换而忽略语境差异

### D. Mixed Cases 预期

1. 礼貌提醒句（draft pending）
   - Tone：`friendly/professional`
   - Clarity：可能建议去冗余短语

2. 提案优化句（good/better/simplify）
   - Vocabulary：应对 `good/better` 给替换
   - Clarity：可能标记表达重复

3. 委员会被动语态句
   - Clarity：应命中 `passive`
   - Tone：偏 `formal/business`

### E. 快速验收检查清单

- 语气检测返回 `overall + score + tones[]` 且分数在 0-100。
- 清晰度结果返回 `score + issues[] + stats`，字段名符合 camelCase。
- 词汇增强返回 `suggestions[]`，每项包含 `original + alternatives[]`。
- “一键应用”前出现确认提示，确认后文本被替换并可接受。
- 长文本触发清晰度流式时，右侧结果会逐步更新（非只在结束后一次显示）。
