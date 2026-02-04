<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Download, Upload, Save, RotateCcw, AlertCircle } from 'lucide-svelte';

	// Rule info from backend
	interface RuleInfo {
		name: string;
		severity: number; // 0=off, 1=error, 2=warning
		description: string;
		defaultSeverity: number;
	}

	// App config from backend
	interface AppConfig {
		rules: Record<string, number>;
		textRules: Record<string, number>;
		spellcheckWords: string[];
		fileTypes: Record<string, string>;
		context: Record<string, number>;
		configPath: string;
	}

	let configPath = '';
	let isLoading = false;
	let saveSuccess = false;
	let loadError: string | null = null;

	// All available rules with their info
	let rules: RuleInfo[] = [];

	// Custom words for spellcheck
	let customWords = '';

	// Hotkey configuration (kept from original UI)
	let hotkeyEnabled = true;
	let customHotkey = 'Cmd+Shift+S';

	// Track unsaved changes
	let hasUnsavedChanges = false;

	async function loadConfiguration() {
		isLoading = true;
		loadError = null;
		try {
			// Load current config and all available rules
			const [config, allRules] = await Promise.all([
				invoke<AppConfig>('get_config'),
				invoke<RuleInfo[]>('get_rules')
			]);

			configPath = config.configPath;
			rules = allRules;

			// Load spellcheck words
			customWords = config.spellcheckWords.join('\n');

			hasUnsavedChanges = false;
		} catch (error) {
			console.error('Failed to load config:', error);
			loadError = error instanceof Error ? error.message : 'Failed to load configuration';
		} finally {
			isLoading = false;
		}
	}

	async function saveConfiguration() {
		isLoading = true;
		loadError = null;
		try {
			// Build rule updates (only include changed rules)
			const rulesUpdate: Record<string, number | null> = {};

			for (const rule of rules) {
				// If severity differs from default, include it; otherwise set to null to reset
				if (rule.severity !== rule.defaultSeverity) {
					rulesUpdate[rule.name] = rule.severity;
				} else {
					rulesUpdate[rule.name] = null;
				}
			}

			// Parse custom words
			const wordsArray = customWords
				.split('\n')
				.map(w => w.trim())
				.filter(w => w.length > 0);

			// Send update to backend
			await invoke('update_config', {
				updates: {
					rules: rulesUpdate,
					spellcheckWords: wordsArray
				}
			});

			// Reload config to reflect saved state
			await loadConfiguration();

			// Show success feedback
			saveSuccess = true;
			setTimeout(() => (saveSuccess = false), 2000);
		} catch (error) {
			console.error('Failed to save config:', error);
			loadError = error instanceof Error ? error.message : 'Failed to save configuration';
		} finally {
			isLoading = false;
		}
	}

	async function exportConfig() {
		try {
			const defaultConfig = await invoke<string>('get_default_config');
			const blob = new Blob([defaultConfig], { type: 'text/yaml' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = '.autocorrectrc';
			a.click();
			URL.revokeObjectURL(url);
		} catch (error) {
			console.error('Failed to export config:', error);
			loadError = 'Failed to export configuration';
		}
	}

	async function importConfig() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = '.autocorrectrc,.yaml,.yml,.txt';

		input.onchange = async (e) => {
			const file = (e.target as HTMLInputElement).files?.[0];
			if (file) {
				try {
					const content = await file.text();
					// Use the existing save_config command to import
					await invoke('save_config', { content });
					// Reload to show new state
					await loadConfiguration();
				} catch (error) {
					console.error('Failed to import config:', error);
					loadError = 'Failed to import configuration';
				}
			}
		};

		input.click();
	}

	async function resetToDefaults() {
		isLoading = true;
		try {
			// Reset all rules to their defaults
			for (const rule of rules) {
				rule.severity = rule.defaultSeverity;
			}
			customWords = '';
			await saveConfiguration();
		} catch (error) {
			console.error('Failed to reset config:', error);
			loadError = 'Failed to reset configuration';
		} finally {
			isLoading = false;
		}
	}

	function getSeverityLabel(severity: number): string {
		switch (severity) {
			case 0: return 'Off';
			case 1: return 'Error';
			case 2: return 'Warning';
			default: return 'Unknown';
		}
	}

	function getSeverityColor(severity: number): string {
		switch (severity) {
			case 0: return 'text-gray-500';
			case 1: return 'text-red-600';
			case 2: return 'text-yellow-600';
			default: return 'text-gray-500';
		}
	}

	function cycleSeverity(rule: RuleInfo) {
		// Cycle: Off (0) -> Error (1) -> Warning (2) -> Off (0)
		rule.severity = (rule.severity + 1) % 3;
		hasUnsavedChanges = true;
	}

	// Load configuration on mount
	loadConfiguration();
</script>

<div class="flex flex-col gap-4 p-6">
	{#if loadError}
		<div class="rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-800 dark:bg-red-950">
			<div class="flex items-start gap-3">
				<AlertCircle class="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<h4 class="text-sm font-semibold text-red-900 dark:text-red-100">Configuration Error</h4>
					<p class="mt-1 text-sm text-red-700 dark:text-red-300">{loadError}</p>
				</div>
				<button
					onclick={() => loadError = null}
					class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-200"
				>
					&times;
				</button>
			</div>
		</div>
	{/if}

	<Card>
		<CardHeader>
			<CardTitle>Settings</CardTitle>
			<CardDescription>
				Configure AutoCorrect rules and preferences. Config path: {configPath || 'Loading...'}
				{#if hasUnsavedChanges}
					<span class="ml-2 text-amber-600 dark:text-amber-400">(Unsaved changes)</span>
				{/if}
			</CardDescription>
		</CardHeader>
		<CardContent class="space-y-6">
			<!-- Rule Toggles with Severity -->
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold">Rules</h3>
					<div class="flex gap-2 text-xs">
						<span class="flex items-center gap-1">
							<span class="h-3 w-3 rounded-full bg-red-600"></span> Error
						</span>
						<span class="flex items-center gap-1">
							<span class="h-3 w-3 rounded-full bg-yellow-600"></span> Warning
						</span>
						<span class="flex items-center gap-1">
							<span class="h-3 w-3 rounded-full bg-gray-400"></span> Off
						</span>
					</div>
				</div>
				<div class="space-y-3">
					{#each rules as rule}
						<div class="flex items-start justify-between rounded-lg border p-3">
							<div class="space-y-0.5 flex-1">
								<div class="flex items-center gap-2">
									<label class="text-sm font-medium">{rule.name}</label>
									<span class="text-xs font-mono {getSeverityColor(rule.severity)}">
										{getSeverityLabel(rule.severity)}
									</span>
									{#if rule.severity !== rule.defaultSeverity}
										<span class="text-xs text-muted-foreground">(default: {getSeverityLabel(rule.defaultSeverity)})</span>
									{/if}
								</div>
								<p class="text-xs text-muted-foreground">{rule.description}</p>
							</div>
							<div class="flex gap-1">
								<button
									onclick={() => { rule.severity = 0; hasUnsavedChanges = true; }}
									class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity === 0 ? 'bg-gray-600 text-white' : 'bg-gray-200 text-gray-700 hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600'}"
									title="Set to Off"
								>
									Off
								</button>
								<button
									onclick={() => { rule.severity = 1; hasUnsavedChanges = true; }}
									class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity === 1 ? 'bg-red-600 text-white' : 'bg-red-200 text-red-700 hover:bg-red-300 dark:bg-red-900 dark:text-red-300 dark:hover:bg-red-800'}"
									title="Set to Error"
								>
									Error
								</button>
								<button
									onclick={() => { rule.severity = 2; hasUnsavedChanges = true; }}
									class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity === 2 ? 'bg-yellow-600 text-white' : 'bg-yellow-200 text-yellow-700 hover:bg-yellow-300 dark:bg-yellow-900 dark:text-yellow-300 dark:hover:bg-yellow-800'}"
									title="Set to Warning"
								>
									Warn
								</button>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<!-- Custom Words -->
			<div class="space-y-3">
				<h3 class="text-sm font-semibold">Custom Spell Check Words</h3>
				<div class="rounded-lg border p-3">
					<label for="custom-words" class="mb-2 block text-sm font-medium">
						Custom Dictionary
					</label>
					<Textarea
						id="custom-words"
						bind:value={customWords}
						oninput={() => hasUnsavedChanges = true}
						placeholder="Enter custom words, one per line&#10;example&#10;customword"
						class="min-h-[100px] font-mono text-sm"
					/>
					<p class="mt-1 text-xs text-muted-foreground">
						Add words that should not be flagged as misspellings, one per line
					</p>
				</div>
			</div>

			<!-- Hotkey Configuration -->
			<div class="space-y-3">
				<h3 class="text-sm font-semibold">Hotkey</h3>
				<div class="flex items-center justify-between rounded-lg border p-3">
					<div class="space-y-0.5">
						<label class="text-sm font-medium" for="hotkey-enabled">Enable Global Hotkey</label>
						<p class="text-xs text-muted-foreground">
							Activate spell check with keyboard shortcut
						</p>
					</div>
					<Switch bind:checked={hotkeyEnabled} id="hotkey-enabled" />
				</div>
				{#if hotkeyEnabled}
					<div class="rounded-lg border p-3">
						<label for="hotkey" class="mb-2 block text-sm font-medium">Custom Hotkey</label>
						<Input
							id="hotkey"
							bind:value={customHotkey}
							placeholder="Cmd+Shift+S"
							class="font-mono"
						/>
						<p class="mt-1 text-xs text-muted-foreground">
							Example: Cmd+Shift+S (macOS) or Ctrl+Shift+S (Windows/Linux)
						</p>
					</div>
				{/if}
			</div>

			<!-- Config Import/Export -->
			<div class="space-y-3">
				<h3 class="text-sm font-semibold">Configuration</h3>
				<div class="flex flex-wrap gap-2">
					<Button onclick={saveConfiguration} variant="default" disabled={isLoading || !hasUnsavedChanges}
						class={saveSuccess ? 'bg-green-600 hover:bg-green-700' : ''}>
						{#if isLoading}
							<div class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></div>
						{:else}
							<Save class="mr-2 h-4 w-4" />
						{/if}
						{saveSuccess ? 'Saved!' : 'Save Changes'}
					</Button>
					<Button onclick={resetToDefaults} variant="outline" disabled={isLoading}>
						<RotateCcw class="mr-2 h-4 w-4" />
						Reset to Defaults
					</Button>
					<Button onclick={exportConfig} variant="outline" disabled={isLoading}>
						<Download class="mr-2 h-4 w-4" />
						Export Default Config
					</Button>
					<Button onclick={importConfig} variant="outline" disabled={isLoading}>
						<Upload class="mr-2 h-4 w-4" />
						Import Config
					</Button>
				</div>
			</div>
		</CardContent>
	</Card>
</div>
