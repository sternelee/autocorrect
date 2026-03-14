<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Check, RefreshCw, Copy } from 'lucide-svelte';

	// Reactive state
	let currentText = '';
	let correctedText = '';
	let isChecking = false;
	let hasChanges = false;
	let lineChanges: Array<{
		line: number;
		col: number;
		original: string;
		corrected: string;
		severity: number;
	}> = [];
	let typos: Array<{
		typo: string;
		suggestions: string[];
		line: number;
		col: number;
	}> = [];
	let aiBusy = false;
	let aiError: string | null = null;
	let aiRunningOperation: 'grammar' | 'translate' | 'polish' | null = null;
	let aiTargetLanguage = 'English';
	let aiPolishStyle = 'professional';
	const translateLanguageOptions = [
		'简体中文',
		'English',
		'繁體中文',
		'日本語',
		'Русский',
		'한국어',
		'Français',
		'Deutsch',
		'Español',
		'Português'
	];

	interface AppConfig {
		aiGrammarEnabled?: boolean;
		aiTranslateTargetLanguage?: string;
		aiPolishStyle?: string;
	}

	async function loadAiDefaults() {
		try {
			const config = await invoke<AppConfig>('get_config');
			aiTargetLanguage = config.aiTranslateTargetLanguage ?? 'English';
			aiPolishStyle = config.aiPolishStyle ?? 'professional';
		} catch (error) {
			console.warn('Failed to load AI defaults:', error);
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

	function applyTypoSuggestion(typo: string, suggestion: string) {
		// Replace typo with suggestion in currentText (case-insensitive, whole word)
		const regex = new RegExp(`\\b${typo}\\b`, 'gi');
		currentText = currentText.replace(regex, suggestion);

		// Update local typo list only; do not trigger spell_check again.
		typos = typos.filter((t) => t.typo !== typo);
	}

	async function addToCustomCorrections(typo: string, correction: string) {
		try {
			await invoke('add_custom_correction', { typo, correction });
			// Remove this typo from the list
			typos = typos.filter(t => t.typo !== typo);
		} catch (error) {
			console.error('Failed to add custom correction:', error);
		}
	}

	async function copyToClipboard() {
		try {
			await invoke('set_clipboard_text', { text: correctedText || currentText });
		} catch (error) {
			console.error('Failed to copy:', error);
		}
	}

	let checkSeq = 0;
	function handleInputKeydown(event: KeyboardEvent) {
		// Enter: run check; Shift+Enter: insert newline
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			performSpellCheck(false);
		}
	}

	async function performSpellCheck(enableAi = false) {
		if (!currentText.trim()) {
			correctedText = '';
			hasChanges = false;
			lineChanges = [];
			typos = [];
			return;
		}

		const seq = ++checkSeq;
		isChecking = true;
		try {
			const result = await invoke<{
				original: string;
				corrected: string;
				has_changes: boolean;
				line_changes: Array<{
					line: number;
					col: number;
					original: string;
					corrected: string;
					severity: number;
				}>;
				typos: Array<{
					typo: string;
					suggestions: string[];
					line: number;
					col: number;
				}>;
			}>('spell_check', { text: currentText, enableAi });

			// Drop stale async result when user keeps typing.
			if (seq !== checkSeq) return;

			correctedText = result.corrected;
			hasChanges = result.has_changes;
			lineChanges = result.line_changes;
			typos = result.typos || [];

			console.log('Spell check result:', { hasChanges, lineChanges: lineChanges.length, typos: typos.length });
		} catch (error) {
			if (seq !== checkSeq) return;
			console.error('Spell check failed:', error);
			correctedText = currentText;
			hasChanges = false;
			typos = [];
		} finally {
			if (seq === checkSeq) {
				isChecking = false;
			}
		}
	}

	async function runAiTransform(operation: 'grammar' | 'translate' | 'polish') {
		if (!currentText.trim() || aiBusy) {
			return;
		}

		aiBusy = true;
		aiError = null;
		aiRunningOperation = operation;
		try {
			const config = await invoke<AppConfig>('get_config');
			if (!config.aiGrammarEnabled) {
				throw new Error('Please enable AI Grammar Check in Settings first.');
			}

			const result = await invoke<{
				outputText?: string;
				typos?: Array<{
					typo: string;
					suggestions: string[];
					line: number;
					col: number;
				}>;
			}>('ai_text_transform', {
				request: {
					text: currentText,
					operation,
					targetLanguage: aiTargetLanguage,
					polishStyle: aiPolishStyle
				}
			});

			if (operation === 'grammar') {
				typos = result.typos || [];
				correctedText = '';
				hasChanges = false;
				lineChanges = [];
			} else {
				typos = [];
				correctedText = result.outputText || '';
				hasChanges = correctedText !== currentText;
				lineChanges = hasChanges
					? [{ line: 1, col: 1, original: currentText, corrected: correctedText, severity: 2 }]
					: [];
			}
		} catch (error) {
			console.error('AI transform failed:', error);
			aiError = error instanceof Error ? error.message : String(error);
		} finally {
			aiBusy = false;
			aiRunningOperation = null;
		}
	}

	loadAiDefaults();
</script>

<div class="flex flex-col gap-4 p-6">
	<Card>
		<CardHeader>
			<CardTitle>Spell Checker</CardTitle>
			<CardDescription>Enter text to check spelling and grammar</CardDescription>
		</CardHeader>
		<CardContent class="space-y-4">
			<!-- Original Text Input -->
			<div class="space-y-2">
				<label for="original-text" class="text-sm font-medium">Original Text</label>
				<Textarea
					id="original-text"
					bind:value={currentText}
					onkeydown={handleInputKeydown}
					placeholder="Type or paste your text here..."
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
						Checking...
					{:else}
						<RefreshCw class="mr-2 h-4 w-4" />
						Check Spelling
					{/if}
				</Button>

				{#if hasChanges}
					<Button onclick={applyCorrection} variant="outline">
						<Check class="mr-2 h-4 w-4" />
						Apply Correction
					</Button>
				{/if}

				<Button onclick={copyToClipboard} variant="ghost" disabled={!currentText}>
					<Copy class="mr-2 h-4 w-4" />
					Copy Result
				</Button>
			</div>

			<div class="rounded-md border p-3 space-y-3">
				<div class="text-sm font-medium">AI Tools</div>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-2">
					<select
						bind:value={aiTargetLanguage}
						class="border-input bg-background ring-offset-background focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-sm outline-none focus-visible:ring-[3px]"
					>
						{#each translateLanguageOptions as language}
							<option value={language}>{language}</option>
						{/each}
					</select>
					<Input bind:value={aiPolishStyle} placeholder="Polish style, e.g. professional / concise / friendly" />
				</div>
				<div class="flex flex-wrap gap-2">
					<Button onclick={() => runAiTransform('grammar')} disabled={aiBusy || !currentText.trim()} variant="outline">
						{aiRunningOperation === 'grammar' ? 'Running...' : 'AI Grammar'}
					</Button>
					<Button onclick={() => runAiTransform('translate')} disabled={aiBusy || !currentText.trim()} variant="outline">
						{aiRunningOperation === 'translate' ? 'Running...' : 'AI Translate'}
					</Button>
					<Button onclick={() => runAiTransform('polish')} disabled={aiBusy || !currentText.trim()} variant="outline">
						{aiRunningOperation === 'polish' ? 'Running...' : 'AI Polish'}
					</Button>
				</div>
				{#if aiError}
					<div class="rounded-md border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700">
						AI Error: {aiError}
					</div>
				{/if}
			</div>

			<!-- Corrected Text Output -->
			{#if correctedText && correctedText !== currentText}
				<div class="space-y-2">
					<label for="corrected-text" class="text-sm font-medium">
						Corrected Text
						{#if hasChanges}
							<span class="ml-2 text-xs text-muted-foreground">
								({lineChanges.length} change{lineChanges.length !== 1 ? 's' : ''} detected)
							</span>
						{/if}
					</label>
					<div
						id="corrected-text"
						class="min-h-[150px] rounded-md border border-input bg-muted/50 p-3 font-mono text-sm whitespace-pre-wrap"
					>
						{correctedText}
					</div>
				</div>
			{/if}

			<!-- Line Changes Detail -->
			{#if lineChanges.length > 0}
				<div class="space-y-2">
					<h3 class="text-sm font-medium">Changes Detected</h3>
					<div class="space-y-2 max-h-[200px] overflow-y-auto">
						{#each lineChanges as change (change.line + ':' + change.col)}
							<div class="flex items-start gap-2 rounded-md border bg-card p-2 text-sm">
								<span class="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs font-mono">
									L{change.line}:C{change.col}
								</span>
								<div class="flex flex-col gap-0.5">
									<span class="text-destructive line-through">{change.original}</span>
									<span class="text-green-600 dark:text-green-400">{change.corrected}</span>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Typos Display -->
			{#if typos.length > 0}
				<div class="space-y-2">
					<h3 class="text-sm font-medium">Spelling Issues ({typos.length})</h3>
					<div class="space-y-2 max-h-[300px] overflow-y-auto">
						{#each typos as typo (typo.typo + ':' + typo.line + ':' + typo.col)}
							<div class="flex flex-col gap-2 rounded-md border border-yellow-400 bg-yellow-50 dark:bg-yellow-950/20 p-3 text-sm">
								<div class="flex items-start justify-between">
									<div class="flex flex-col gap-1">
										<span class="font-semibold text-red-600 dark:text-red-400">
											"{typo.typo}"
										</span>
										<span class="text-xs text-muted-foreground">
											Line {typo.line}, Column {typo.col}
										</span>
									</div>
								</div>
								
								{#if typo.suggestions.length > 0}
									<div class="flex flex-col gap-2">
										<span class="text-xs font-medium text-muted-foreground">Suggestions:</span>
										<div class="flex flex-wrap gap-2">
											{#each typo.suggestions.slice(0, 5) as suggestion}
												<button
													onclick={() => applyTypoSuggestion(typo.typo, suggestion)}
													class="rounded bg-green-600 px-2 py-1 text-xs text-white hover:bg-green-700 dark:bg-green-700 dark:hover:bg-green-600"
												>
													{suggestion}
												</button>
											{/each}
										</div>
										{#if typo.suggestions.length > 0}
											<button
												onclick={() => addToCustomCorrections(typo.typo, typo.suggestions[0])}
												class="self-start rounded bg-blue-600 px-2 py-1 text-xs text-white hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600"
											>
												Add "{typo.typo}" → "{typo.suggestions[0]}" to Custom Corrections
											</button>
										{/if}
									</div>
								{:else}
									<span class="text-xs text-muted-foreground">No suggestions available</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</CardContent>
	</Card>
</div>
