<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Download, Upload, Save, RotateCcw, AlertCircle, Keyboard } from 'lucide-svelte';

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

	// Hotkey configuration
	interface Modifiers {
		shift: boolean;
		ctrl: boolean;
		meta: boolean;
		alt: boolean;
	}

	interface HotkeyConfig {
		key: string;
		modifiers: Modifiers;
		display_string: string;
	}

	let configPath = '';
	let isLoading = false;
	let saveSuccess = false;
	let loadError: string | null = null;

	// All available rules with their info
	let rules: RuleInfo[] = [];

	// Custom words for spellcheck
	let customWords = '';

	// Hotkey configuration state
	let hotkeyEnabled = true;
	let hotkeyConfig: HotkeyConfig | null = null;
	let showKeySelector = false;
	let isRecording = false;
	let recordedShortcut: HotkeyConfig | null = null;
	let recordingError: string | null = null;
	let recordingTimeout: ReturnType<typeof setTimeout> | null = null;

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
	loadHotkeyConfiguration();

	async function loadHotkeyConfiguration() {
		try {
			const config = await invoke<HotkeyConfig>('get_hotkey_config');
			hotkeyConfig = config;
		} catch (error) {
			console.error('Failed to load hotkey config:', error);
		}
	}

	async function startRecording() {
		isRecording = true;
		recordingError = null;
		recordedShortcut = null;

		// Clear any existing timeout
		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
		}

		// Set a timeout to auto-cancel recording after 10 seconds
		recordingTimeout = setTimeout(() => {
			if (isRecording) {
				isRecording = false;
				recordingError = 'Recording timed out. Please try again.';
				// Remove keyboard listener
				document.removeEventListener('keydown', handleKeyPress);
			}
		}, 10000);

		// Add keyboard event listener
		document.addEventListener('keydown', handleKeyPress);
	}

	function handleKeyPress(e: KeyboardEvent) {
		if (!isRecording) {
			document.removeEventListener('keydown', handleKeyPress);
			return;
		}

		// Ignore modifier keys by themselves - we only want the final key
		// The modifiers are captured from e.shiftKey, e.metaKey, etc.
		const isModifierKey = e.key === 'Shift' || e.key === 'Control' ||
		                   e.key === 'Meta' || e.key === 'Alt' ||
		                   e.code === 'ShiftLeft' || e.code === 'ShiftRight' ||
		                   e.code === 'ControlLeft' || e.code === 'ControlRight' ||
		                   e.code === 'MetaLeft' || e.code === 'MetaRight' ||
		                   e.code === 'AltLeft' || e.code === 'AltRight';

		if (isModifierKey) {
			// Don't prevent default for modifier keys, just ignore them
			return;
		}

		e.preventDefault();
		e.stopPropagation();

		// Clear timeout
		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
		}

		// Build modifiers from the event
		const modifiers: Modifiers = {
			shift: e.shiftKey,
			ctrl: e.ctrlKey,
			meta: e.metaKey,
			alt: e.altKey
		};

		// Map keyboard event key to our key names
		let keyName = mapEventKeyToKeyName(e.key, e.code);

		if (!keyName) {
			recordingError = `Unsupported key: "${e.key}" (code: ${e.code}). Please use A-Z, F1-F12, Space, Enter, Tab, or other standard keys.`;
			isRecording = false;
			document.removeEventListener('keydown', handleKeyPress);
			return;
		}

		// Create the recorded shortcut
		recordedShortcut = {
			key: keyName,
			modifiers,
			display_string: formatShortcutDisplay(modifiers, keyName)
		};

		isRecording = false;
		document.removeEventListener('keydown', handleKeyPress);
	}

	function mapEventKeyToKeyName(key: string, code: string): string | null {
		// Map event key/code to our key names
		// Prefer code over key for letter keys as it's more consistent
		if (code === 'Space' || key === ' ') return 'Space';
		if (code === 'Enter' || key === 'Enter') return 'Return';
		if (code === 'Tab' || key === 'Tab') return 'Tab';
		if (code === 'Backspace' || key === 'Backspace') return 'Backspace';
		if (code === 'Delete' || key === 'Delete') return 'Backspace';
		if (code === 'Escape' || key === 'Escape') return 'Escape';

		// Function keys - use key as it's more reliable
		if (key.startsWith('F') && key.length <= 3) {
			const num = parseInt(key.substring(1));
			if (num >= 1 && num <= 12) return key;
		}

		// Letter keys - use code which is consistent (e.g., "KeyK" for 'k' key)
		if (code.startsWith('Key') && code.length === 4) {
			return code; // e.g., "KeyK" -> "KeyK"
		}

		// Fallback: try using key for letters
		if (/^[a-zA-Z]$/.test(key)) {
			return 'Key' + key.toUpperCase();
		}

		// Number keys
		if (/^[0-9]$/.test(key)) {
			return 'Num' + key;
		}
		if (code.startsWith('Digit')) {
			return 'Num' + code.substring(5);
		}

		return null;
	}

	function formatShortcutDisplay(modifiers: Modifiers, key: string): string {
		const parts: string[] = [];

		if (modifiers.meta) parts.push('⌘');
		if (modifiers.shift) parts.push('⇧');
		if (modifiers.alt) parts.push('⌥');
		if (modifiers.ctrl) parts.push('⌃');

		// Format the key nicely
		let keyLabel = key;
		if (key === 'Space') keyLabel = 'Space';
		else if (key === 'Return') keyLabel = 'Return';
		else if (key === 'Tab') keyLabel = 'Tab';
		else if (key === 'Backspace') keyLabel = '⌫';
		else if (key === 'Escape') keyLabel = 'Esc';
		else if (key.startsWith('Key')) keyLabel = key.substring(3);
		else if (key.startsWith('Num')) keyLabel = key.substring(3);
		else if (key.startsWith('F')) keyLabel = key;

		parts.push(keyLabel);
		return parts.join('+');
	}

	async function saveRecordedShortcut() {
		if (!recordedShortcut) return;

		try {
			const newConfig = await invoke<HotkeyConfig>('update_hotkey_config', {
				request: {
					key: recordedShortcut.key,
					modifiers: recordedShortcut.modifiers
				}
			});
			hotkeyConfig = newConfig;
			showKeySelector = false;
			recordedShortcut = null;
		} catch (error) {
			console.error('Failed to save hotkey config:', error);
			loadError = 'Failed to save hotkey configuration';
		}
	}

	async function resetHotkeyToDefaults() {
		try {
			const defaultConfig = await invoke<HotkeyConfig>('reset_hotkey_config');
			hotkeyConfig = defaultConfig;
			showKeySelector = false;
		} catch (error) {
			console.error('Failed to reset hotkey config:', error);
			loadError = 'Failed to reset hotkey configuration';
		}
	}

	function cancelRecording() {
		isRecording = false;
		recordedShortcut = null;
		recordingError = null;
		if (recordingTimeout) {
			clearTimeout(recordingTimeout);
			recordingTimeout = null;
		}
		// Remove keyboard listener
		document.removeEventListener('keydown', handleKeyPress);
	}
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
				<h3 class="text-sm font-semibold">Global Hotkey</h3>
				<div class="flex items-center justify-between rounded-lg border p-3">
					<div class="space-y-0.5">
						<label class="text-sm font-medium" for="hotkey-enabled">Enable Global Hotkey</label>
						<p class="text-xs text-muted-foreground">
							Activate spell check with keyboard shortcut from any application
						</p>
					</div>
					<Switch bind:checked={hotkeyEnabled} id="hotkey-enabled" />
				</div>

				{#if hotkeyEnabled}
					<!-- Current Hotkey Display -->
					<div class="rounded-lg border p-3">
						<div class="mb-3 flex items-center justify-between">
							<label class="text-sm font-medium">Current Hotkey</label>
							{#if hotkeyConfig}
								<div class="flex items-center gap-2 rounded-md bg-muted px-3 py-1.5 font-mono text-sm">
									<Keyboard class="h-4 w-4" />
									<span>{hotkeyConfig.display_string}</span>
								</div>
							{:else}
								<div class="text-sm text-muted-foreground">Loading...</div>
							{/if}
						</div>

						<!-- Change Hotkey Button -->
						{#if !showKeySelector}
							<div class="flex gap-2">
								<button
									onclick={() => showKeySelector = true}
									class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
								>
									Change Hotkey
								</button>
								<button
									onclick={resetHotkeyToDefaults}
									class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
								>
									Reset to Default
								</button>
							</div>
						{/if}
					</div>

					<!-- Shortcut Recording UI -->
					{#if showKeySelector}
						<div class="space-y-3 rounded-lg border p-3">
							<h4 class="text-sm font-medium">Change Hotkey</h4>

							{#if !isRecording && !recordedShortcut}
								<!-- Recording Instructions -->
								<div class="rounded-md bg-muted p-4 text-center">
									<p class="text-sm text-muted-foreground">
										Press your desired key combination to record it.
										<br />
										For example: <span class="font-mono">⌘+⇧+S</span>
									</p>
									<button
										onclick={startRecording}
										class="mt-3 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
									>
										<Keyboard class="mr-2 h-4 w-4 inline" />
										Start Recording
									</button>
								</div>
							{/if}

							{#if isRecording}
								<!-- Recording State -->
								<div class="rounded-md border-2 border-primary bg-primary/5 p-6 text-center">
									<div class="mb-3 flex justify-center">
										<div class="flex h-8 w-8 animate-pulse items-center justify-center rounded-full bg-primary">
											<Keyboard class="h-4 w-4 text-primary-foreground" />
										</div>
									</div>
									<p class="text-sm font-medium">Recording...</p>
									<p class="mt-1 text-xs text-muted-foreground">
										Press your key combination now. Recording will timeout in 10 seconds.
									</p>
									<button
										onclick={cancelRecording}
										class="mt-3 rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
									>
										Cancel
									</button>
								</div>
							{/if}

							{#if recordedShortcut}
								<!-- Recorded Result -->
								<div class="rounded-md bg-muted p-4 text-center">
									<p class="text-xs font-medium text-muted-foreground">Recorded:</p>
									<p class="mt-2 font-mono text-lg">
										{recordedShortcut.display_string}
									</p>
									<div class="mt-4 flex justify-center gap-2">
										<button
											onclick={saveRecordedShortcut}
											class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
										>
											Save Hotkey
										</button>
										<button
											onclick={() => recordedShortcut = null}
											class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
										>
											Try Again
										</button>
									</div>
								</div>
							{/if}

							{#if recordingError}
								<!-- Error State -->
								<div class="rounded-md border border-red-200 bg-red-50 p-4 text-center dark:border-red-800 dark:bg-red-950">
									<p class="text-sm font-medium text-red-900 dark:text-red-100">
										{recordingError}
									</p>
									<div class="mt-3 flex justify-center gap-2">
										<button
											onclick={() => {
												recordingError = null;
												startRecording();
											}}
											class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
										>
											Try Again
										</button>
										<button
											onclick={() => {
												recordingError = null;
												showKeySelector = false;
											}}
											class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
										>
											Cancel
										</button>
									</div>
								</div>
							{/if}

							<!-- Cancel Button (bottom) -->
							{#if !isRecording && !recordedShortcut && !recordingError}
								<button
									onclick={() => showKeySelector = false}
									class="w-full rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
								>
									Cancel
								</button>
							{/if}
						</div>
					{/if}
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
