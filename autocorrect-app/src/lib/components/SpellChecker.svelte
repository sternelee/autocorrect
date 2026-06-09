<script lang="ts">
  $locale;
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";
  import { Textarea } from "$lib/components/ui/textarea";
  import { Check, RefreshCw, Copy } from "lucide-svelte";
  import { locale, t } from "$lib/i18n";
  import type { AppConfig, LineChange, TypoSuggestion, SpellCheckResult, AiTextTransformResponse, AiClarityCheckResponse, AiVocabularyEnhanceResponse } from "$lib/types/app";

  // Reactive translation helper
  const tr = $derived(
    (key: string, params?: Record<string, string | number>) => {
      const _ = $locale;
      return t(key, params);
    },
  );

  // Reactive state
  let currentText = $state("");
  let correctedText = $state("");
  let isChecking = $state(false);
  let hasChanges = $state(false);
  let lineChanges: LineChange[] = $state([]);
  let typos: TypoSuggestion[] = $state([]);
  let aiBusy = $state(false);
  let aiError: string | null = $state(null);
  let aiRunningOperation:
    | "grammar"
    | "translate"
    | "polish"
    | "summarize"
    | "clarity"
    | "vocabulary"
    | null = $state(null);
  let clarityResult: AiClarityCheckResponse | null = $state(null);
  let vocabularyResult: AiVocabularyEnhanceResponse | null = $state(null);
  let aiTargetLanguage = $state("English");
  let aiPolishStyles = $state<string[]>([]);
  let unlistenChunk: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;

  const translateLanguageOptions = [
    "简体中文",
    "English",
    "繁體中文",
    "日本語",
    "Русский",
    "한국어",
    "Français",
    "Deutsch",
    "Español",
    "Português",
  ];

  async function loadAiDefaults() {
    try {
      const config = await invoke<AppConfig>("get_config");
      aiTargetLanguage = config.aiTranslateTargetLanguage ?? "English";
      aiPolishStyles = config.aiPolishStyle?.length
        ? config.aiPolishStyle
        : config.aiPolishStyles?.length
          ? config.aiPolishStyles
          : ["formal"];
    } catch (error) {
      console.warn("Failed to load AI defaults:", error);
    }
  }

  async function applyCorrection() {
    if (hasChanges && correctedText) {
      currentText = correctedText;
      hasChanges = false;
      lineChanges = [];
      typos = [];
    }
  }

  function escapeRegExp(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function applyTypoSuggestion(typo: string, suggestion: string) {
    // Replace typo with suggestion in currentText (case-insensitive, whole word)
    const regex = new RegExp(`\\b${escapeRegExp(typo)}\\b`, "gi");
    currentText = currentText.replace(regex, suggestion);

    // Update local typo list only; do not trigger spell_check again.
    typos = typos.filter((t) => t.typo !== typo);
  }

  async function addToCustomCorrections(typo: string, correction: string) {
    try {
      await invoke("add_custom_correction", { typo, correction });
      // Remove this typo from the list
      typos = typos.filter((t) => t.typo !== typo);
    } catch (error) {
      console.error("Failed to add custom correction:", error);
    }
  }

  async function copyToClipboard() {
    try {
      await invoke("set_clipboard_text", {
        text: correctedText || currentText,
      });
    } catch (error) {
      console.error("Failed to copy:", error);
    }
  }

  let checkSeq = $state(0);
  function handleInputKeydown(event: KeyboardEvent) {
    // Enter: run check; Shift+Enter: insert newline
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      performSpellCheck(false);
    }
  }

  async function performSpellCheck(enableAi = false) {
    if (!currentText.trim()) {
      correctedText = "";
      hasChanges = false;
      lineChanges = [];
      typos = [];
      return;
    }

    const seq = ++checkSeq;
    isChecking = true;
    try {
      const result = await invoke<SpellCheckResult>("spell_check", { text: currentText, enableAi });

      // Drop stale async result when user keeps typing.
      if (seq !== checkSeq) return;

      correctedText = result.corrected;
      hasChanges = result.has_changes;
      lineChanges = result.line_changes;
      typos = result.typos || [];

      console.log("Spell check result:", {
        hasChanges,
        lineChanges: lineChanges.length,
        typos: typos.length,
      });
    } catch (error) {
      if (seq !== checkSeq) return;
      console.error("Spell check failed:", error);
      correctedText = currentText;
      hasChanges = false;
      typos = [];
    } finally {
      if (seq === checkSeq) {
        isChecking = false;
      }
    }
  }

  function buildAiRequest(
    operation: "grammar" | "translate" | "polish" | "summarize",
  ) {
    const polishStyle = aiPolishStyles[0] ?? "formal";
    return {
      text: currentText,
      operation: operation === "summarize" ? "polish" : operation,
      targetLanguage: aiTargetLanguage,
      polishStyle:
        operation === "polish" || operation === "summarize"
          ? polishStyle
          : polishStyle,
    };
  }

  async function runAiTransform(
    operation: "grammar" | "translate" | "polish" | "summarize",
  ) {
    if (!currentText.trim() || aiBusy) {
      return;
    }

    // Clear non-transform results when running a transform operation.
    clarityResult = null;
    vocabularyResult = null;

    if (operation !== "grammar") {
      await runAiTransformStream(operation);
      return;
    }

    aiBusy = true;
    aiError = null;
    aiRunningOperation = operation;
    try {
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("spell.aiEnableHint"));
      }

      const result = await invoke<AiTextTransformResponse>("ai_text_transform", {
        request: buildAiRequest(operation),
      });

      if (operation === "grammar") {
        typos = result.typos || [];
        correctedText = currentText;
        hasChanges = false;
        lineChanges = [];
      } else {
        typos = [];
        correctedText = result.outputText || "";
        hasChanges = correctedText !== currentText;
        lineChanges = hasChanges
          ? [
              {
                line: 1,
                col: 1,
                original: currentText,
                corrected: correctedText,
                severity: 2,
              },
            ]
          : [];
      }
    } catch (error) {
      console.error("AI transform failed:", error);
      aiError = error instanceof Error ? error.message : String(error);
    } finally {
      aiBusy = false;
      aiRunningOperation = null;
    }
  }

  async function runAiClarityCheck() {
    if (!currentText.trim() || aiBusy) return;
    aiBusy = true;
    aiError = null;
    aiRunningOperation = "clarity";
    clarityResult = null;
    vocabularyResult = null;
    try {
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("spell.aiEnableHint"));
      }
      const result = await invoke<AiClarityCheckResponse>("ai_clarity_check", {
        request: { text: currentText },
      });
      clarityResult = result;
    } catch (error) {
      console.error("AI clarity check failed:", error);
      aiError = error instanceof Error ? error.message : String(error);
    } finally {
      aiBusy = false;
      aiRunningOperation = null;
    }
  }

  async function runAiVocabularyEnhance() {
    if (!currentText.trim() || aiBusy) return;
    aiBusy = true;
    aiError = null;
    aiRunningOperation = "vocabulary";
    vocabularyResult = null;
    clarityResult = null;
    try {
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("spell.aiEnableHint"));
      }
      const result = await invoke<AiVocabularyEnhanceResponse>("ai_vocabulary_enhance", {
        request: { text: currentText },
      });
      vocabularyResult = result;
    } catch (error) {
      console.error("AI vocabulary enhance failed:", error);
      aiError = error instanceof Error ? error.message : String(error);
    } finally {
      aiBusy = false;
      aiRunningOperation = null;
    }
  }

  async function runAiTransformStream(
    operation: "grammar" | "translate" | "polish" | "summarize",
  ) {
    if (!currentText.trim() || aiBusy) {
      return;
    }

    aiBusy = true;
    aiError = null;
    aiRunningOperation = operation;

    try {
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("spell.aiEnableHint"));
      }

      typos = [];
      correctedText = "";

      await invoke("ai_text_transform_stream", {
        request: buildAiRequest(operation),
      });
    } catch (error) {
      console.error("AI stream transform failed:", error);
      aiError = error instanceof Error ? error.message : String(error);
    } finally {
      aiBusy = false;
      aiRunningOperation = null;
    }
  }

  onMount(async () => {
    unlistenChunk = await listen<string>("ai-stream-chunk", (event) => {
      if (!aiBusy || !aiRunningOperation) {
        return;
      }
      correctedText += event.payload;
    });

    unlistenComplete = await listen("ai-stream-complete", () => {
      if (!aiBusy || !aiRunningOperation) {
        return;
      }
      if (aiRunningOperation !== "grammar") {
        hasChanges = correctedText !== currentText;
        lineChanges = hasChanges
          ? [
              {
                line: 1,
                col: 1,
                original: currentText,
                corrected: correctedText,
                severity: 2,
              },
            ]
          : [];
      }
      aiBusy = false;
      aiRunningOperation = null;
    });

    unlistenError = await listen<string>("ai-stream-error", (event) => {
      if (!aiBusy || !aiRunningOperation) {
        return;
      }
      aiError = event.payload;
      aiBusy = false;
      aiRunningOperation = null;
    });
  });

  onDestroy(() => {
    if (unlistenChunk) unlistenChunk();
    if (unlistenComplete) unlistenComplete();
    if (unlistenError) unlistenError();
  });

  loadAiDefaults();
</script>

<div class="flex flex-col gap-4 p-6" data-locale={$locale}>
  <Card>
    <CardHeader>
      <CardTitle>{tr("spell.title")}</CardTitle>
      <CardDescription>{tr("spell.desc")}</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
      <!-- Original Text Input -->
      <div class="space-y-2">
        <label for="original-text" class="text-sm font-medium"
          >{tr("spell.original")}</label
        >
        <Textarea
          id="original-text"
          bind:value={currentText}
          onkeydown={handleInputKeydown}
          placeholder={tr("spell.placeholder")}
          class="min-h-[150px] font-mono text-sm"
        />
      </div>

      <!-- Action Buttons -->
      <div class="flex flex-wrap gap-2">
        <Button
          onclick={() => performSpellCheck(false)}
          disabled={isChecking || !currentText.trim()}
          variant="default"
        >
          {#if isChecking}
            <RefreshCw class="mr-2 h-4 w-4 animate-spin" />
            {tr("spell.checking")}
          {:else}
            <RefreshCw class="mr-2 h-4 w-4" />
            {tr("spell.check")}
          {/if}
        </Button>

        {#if hasChanges}
          <Button onclick={applyCorrection} variant="outline">
            <Check class="mr-2 h-4 w-4" />
            {tr("spell.apply")}
          </Button>
        {/if}

        <Button
          onclick={copyToClipboard}
          variant="ghost"
          disabled={!currentText}
        >
          <Copy class="mr-2 h-4 w-4" />
          {tr("spell.copy")}
        </Button>
      </div>

      <div class="space-y-3 rounded-md border p-3">
        <div class="text-sm font-medium">{tr("spell.aiTools")}</div>
        <div class="flex">
          <select
            bind:value={aiTargetLanguage}
            class="border-input bg-background ring-offset-background focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-sm outline-none focus-visible:ring-[3px]"
          >
            {#each translateLanguageOptions as language}
              <option value={language}>{language}</option>
            {/each}
          </select>
        </div>
        <div class="flex flex-wrap gap-2">
          <Button
            onclick={() => runAiTransform("grammar")}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "grammar"
              ? tr("spell.running")
              : tr("spell.aiGrammar")}
          </Button>
          <Button
            onclick={() => runAiTransform("translate")}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "translate"
              ? tr("spell.running")
              : tr("spell.aiTranslate")}
          </Button>
          <Button
            onclick={() => runAiTransform("polish")}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "polish"
              ? tr("spell.running")
              : tr("spell.aiPolish")}
          </Button>
          <Button
            onclick={() => runAiTransform("summarize")}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "summarize"
              ? tr("spell.running")
              : tr("spell.aiSummarize")}
          </Button>
          <Button
            onclick={runAiClarityCheck}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "clarity"
              ? tr("spell.running")
              : tr("spell.aiClarity")}
          </Button>
          <Button
            onclick={runAiVocabularyEnhance}
            disabled={aiBusy || !currentText.trim()}
            variant="outline"
          >
            {aiRunningOperation === "vocabulary"
              ? tr("spell.running")
              : tr("spell.aiVocabulary")}
          </Button>
        </div>
        {#if aiError}
          <div
            class="rounded-md border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700"
          >
            {tr("spell.aiError")}: {aiError}
          </div>
        {/if}
      </div>

      <!-- Corrected Text Output -->
      {#if correctedText && correctedText !== currentText}
        <div class="space-y-2">
          <label for="corrected-text" class="text-sm font-medium">
            {tr("spell.corrected")}
            {#if hasChanges}
              <span class="text-muted-foreground ml-2 text-xs">
                ({lineChanges.length}
                {lineChanges.length === 1
                  ? tr("spell.changeDetected")
                  : tr("spell.changesDetected")})
              </span>
            {/if}
          </label>
          <div
            id="corrected-text"
            class="border-input bg-muted/50 min-h-[150px] rounded-md border p-3 font-mono text-sm whitespace-pre-wrap"
          >
            {correctedText}
          </div>
        </div>
      {/if}

      <!-- Line Changes Detail -->
      {#if lineChanges.length > 0}
        <div class="space-y-2">
          <h3 class="text-sm font-medium">{tr("spell.changesTitle")}</h3>
          <div class="max-h-[200px] space-y-2 overflow-y-auto">
            {#each lineChanges as change (change.line + ":" + change.col)}
              <div
                class="bg-card flex items-start gap-2 rounded-md border p-2 text-sm"
              >
                <span
                  class="bg-muted shrink-0 rounded px-1.5 py-0.5 font-mono text-xs"
                >
                  L{change.line}:C{change.col}
                </span>
                <div class="flex flex-col gap-0.5">
                  <span class="text-destructive line-through"
                    >{change.original}</span
                  >
                  <span class="text-green-600 dark:text-green-400"
                    >{change.corrected}</span
                  >
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Typos Display -->
      {#if typos.length > 0}
        <div class="space-y-2">
          <h3 class="text-sm font-medium">
            {tr("spell.spellingIssues")} ({typos.length})
          </h3>
          <div class="max-h-[300px] space-y-2 overflow-y-auto">
            {#each typos as typo (typo.typo + ":" + typo.line + ":" + typo.col)}
              <div
                class="flex flex-col gap-2 rounded-md border border-yellow-400 bg-yellow-50 p-3 text-sm dark:bg-yellow-950/20"
              >
                <div class="flex items-start justify-between">
                  <div class="flex flex-col gap-1">
                    <span class="font-semibold text-red-600 dark:text-red-400">
                      "{typo.typo}"
                    </span>
                    <span class="text-muted-foreground text-xs">
                      {tr("spell.line")}
                      {typo.line}, {tr("spell.column")}
                      {typo.col}
                    </span>
                  </div>
                </div>

                {#if typo.suggestions.length > 0}
                  <div class="flex flex-col gap-2">
                    <span class="text-muted-foreground text-xs font-medium"
                      >{tr("spell.suggestions")}</span
                    >
                    <div class="flex flex-wrap gap-2">
                      {#each typo.suggestions.slice(0, 5) as suggestion}
                        <button
                          onclick={() =>
                            applyTypoSuggestion(typo.typo, suggestion)}
                          class="rounded bg-green-600 px-2 py-1 text-xs text-white hover:bg-green-700 dark:bg-green-700 dark:hover:bg-green-600"
                        >
                          {suggestion}
                        </button>
                      {/each}
                    </div>
                    {#if typo.suggestions.length > 0}
                      <button
                        onclick={() =>
                          addToCustomCorrections(
                            typo.typo,
                            typo.suggestions[0],
                          )}
                        class="self-start rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600"
                      >
                        {tr("spell.addCustom", {
                          typo: typo.typo,
                          suggestion: typo.suggestions[0],
                        })}
                      </button>
                    {/if}
                  </div>
                {:else}
                  <span class="text-muted-foreground text-xs"
                    >{tr("spell.noSuggestions")}</span
                  >
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- AI Clarity Result -->
      {#if clarityResult}
        <div class="space-y-2">
          <h3 class="text-sm font-medium">
            {tr("aipopup.analysis.clarityTitle")}
            {#if clarityResult.score > 0}
              <span class="text-muted-foreground ml-2 text-xs">
                {tr("aipopup.analysis.score")}: {clarityResult.score}
              </span>
            {/if}
          </h3>
          {#if clarityResult.stats}
            <div class="flex flex-wrap gap-3 text-xs text-muted-foreground">
              <span>{tr("aipopup.analysis.readability")}: {clarityResult.stats.readabilityGrade}</span>
              <span>{tr("aipopup.analysis.avgSentence")}: {clarityResult.stats.avgSentenceLength.toFixed(1)}</span>
              <span>{tr("aipopup.analysis.passiveVoice")}: {clarityResult.stats.passiveVoiceCount}</span>
            </div>
          {/if}
          {#if clarityResult.issues.length > 0}
            <div class="max-h-[300px] space-y-2 overflow-y-auto">
              {#each clarityResult.issues as issue}
                <div class="flex flex-col gap-1 rounded-md border p-3 text-sm">
                  <div class="flex items-center gap-2">
                    <span class="rounded bg-amber-100 px-1.5 py-0.5 text-xs font-medium text-amber-800 dark:bg-amber-900/30 dark:text-amber-300">
                      {tr("aipopup.analysis.issue." + issue.issueType.toLowerCase()) || issue.issueType}
                    </span>
                    <span class="text-muted-foreground text-xs">
                      L{issue.line}:C{issue.col}
                    </span>
                  </div>
                  <span class="text-red-600 dark:text-red-400">"{issue.text}"</span>
                  <span class="text-green-600 dark:text-green-400">{tr("aipopup.analysis.suggestion")}: {issue.suggestion}</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-muted-foreground">{tr("aipopup.analysis.noIssues")}</p>
          {/if}
        </div>
      {/if}

      <!-- AI Vocabulary Result -->
      {#if vocabularyResult}
        <div class="space-y-2">
          <h3 class="text-sm font-medium">
            {tr("aipopup.analysis.vocabTitle")}
          </h3>
          {#if vocabularyResult.suggestions.length > 0}
            <div class="max-h-[300px] space-y-2 overflow-y-auto">
              {#each vocabularyResult.suggestions as vocab}
                <div class="flex flex-col gap-2 rounded-md border p-3 text-sm">
                  <div class="flex items-center gap-2">
                    <span class="font-semibold text-red-600 dark:text-red-400">
                      "{vocab.original}"
                    </span>
                    <span class="text-muted-foreground text-xs">
                      L{vocab.line}:C{vocab.col}
                    </span>
                  </div>
                  {#if vocab.alternatives.length > 0}
                    <div class="flex flex-wrap gap-2">
                      {#each vocab.alternatives.slice(0, 3) as alt}
                        <button
                          onclick={() => applyTypoSuggestion(vocab.original, alt.word)}
                          class="rounded bg-purple-600 px-2 py-1 text-xs text-white hover:bg-purple-700 dark:bg-purple-700 dark:hover:bg-purple-600"
                          title={alt.reason}
                        >
                          {alt.word}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-muted-foreground">{tr("aipopup.analysis.vocabNoSuggestions")}</p>
          {/if}
        </div>
      {/if}
    </CardContent>
  </Card>
</div>
