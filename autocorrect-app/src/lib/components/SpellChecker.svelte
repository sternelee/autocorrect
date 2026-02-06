<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
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

	async function performSpellCheck() {
		if (!currentText.trim()) {
			correctedText = '';
			hasChanges = false;
			lineChanges = [];
			typos = [];
			return;
		}

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
			}>('spell_check', { text: currentText });

			correctedText = result.corrected;
			hasChanges = result.has_changes;
			lineChanges = result.line_changes;
			typos = result.typos || [];
			
			console.log('Spell check result:', { hasChanges, lineChanges: lineChanges.length, typos: typos.length });
		} catch (error) {
			console.error('Spell check failed:', error);
			correctedText = currentText;
			hasChanges = false;
			typos = [];
		} finally {
			isChecking = false;
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
		
		// Re-run spell check to update results
		performSpellCheck();
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

	// Auto-run spell check when text changes (debounced)
	let checkTimeout: ReturnType<typeof setTimeout> | undefined;
	function handleInput() {
		if (checkTimeout) clearTimeout(checkTimeout);
		checkTimeout = setTimeout(() => {
			performSpellCheck();
		}, 500);
	}
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
					oninput={handleInput}
					placeholder="Type or paste your text here..."
					class="min-h-[150px] font-mono text-sm"
				/>
			</div>

			<!-- Action Buttons -->
			<div class="flex flex-wrap gap-2">
				<Button
					onclick={performSpellCheck}
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
