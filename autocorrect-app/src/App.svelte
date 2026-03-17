<script lang="ts">
	$locale;
	import SpellChecker from '$lib/components/SpellChecker.svelte';
	import SettingsPanel from '$lib/components/SettingsPanel.svelte';
	import StatusIndicator from '$lib/components/StatusIndicator.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Settings, Home, Info } from 'lucide-svelte';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';
	import { locale, t } from '$lib/i18n';
	$locale;

	// Reactive translation helper
	const tr = $derived((key: string, params?: Record<string, string | number>) => {
		const _ = $locale;
		return t(key, params);
	});

	// App state
	let currentTab: 'spellchecker' | 'settings' | 'about' = $state('spellchecker');
	let isEnabled = $state(true);
	let correctionCount = $state(0);

	// Theme management
	type ThemeMode = 'light' | 'dark' | 'auto';
	const THEME_STORAGE_KEY = 'autocorrect-theme';
	let theme: ThemeMode = $state('auto');
	let mediaQuery: MediaQueryList | null = null;

	function loadTheme(): ThemeMode {
		const stored = localStorage.getItem(THEME_STORAGE_KEY);
		if (stored === 'light' || stored === 'dark' || stored === 'auto') {
			return stored;
		}
		return 'auto';
	}

	function applyTheme(mode: ThemeMode) {
		const html = document.documentElement;
		if (mode === 'dark') {
			html.classList.add('dark');
		} else if (mode === 'auto') {
			const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
			html.classList.toggle('dark', prefersDark);
		} else {
			html.classList.remove('dark');
		}
		localStorage.setItem(THEME_STORAGE_KEY, mode);
	}

	function setupSystemThemeListener() {
		if (mediaQuery) {
			mediaQuery.removeEventListener('change', handleSystemThemeChange);
		}
		mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		mediaQuery.addEventListener('change', handleSystemThemeChange);
	}

	function handleSystemThemeChange(e: MediaQueryListEvent) {
		if (theme === 'auto') {
			const html = document.documentElement;
			html.classList.toggle('dark', e.matches);
		}
	}

	function cleanupThemeListener() {
		if (mediaQuery) {
			mediaQuery.removeEventListener('change', handleSystemThemeChange);
			mediaQuery = null;
		}
	}

	function handleToggleEnabled(enabled: boolean) {
		isEnabled = enabled;
	}

	// Apply theme when it changes
	$effect(() => {
		applyTheme(theme);
	});

	// Listen for Tauri events from the Rust backend
	onMount(() => {
		// Initialize theme
		theme = loadTheme();
		applyTheme(theme);
		setupSystemThemeListener();

		// Listen for suggestion-accepted event (from popup window)
		const unlistenAccepted = listen('suggestion-accepted', () => {
			correctionCount++;
		});

		// Listen for no-changes-needed notification
		const unlistenNoChanges = listen('no-changes-needed', () => {
			console.log(tr('spell.noSuggestions'));
			// Could show a small toast notification here
		});

		// Cleanup on destroy
		return () => {
			unlistenAccepted.then((unlisten) => unlisten());
			unlistenNoChanges.then((unlisten) => unlisten());
		};
	});

	onDestroy(() => {
		cleanupThemeListener();
	});
</script>

<div class="flex h-screen flex-col bg-background" data-locale={$locale}>
	<!-- Top Status Bar -->
	<header class="border-b bg-card/50 backdrop-blur-sm">
		<div class="flex items-center justify-between px-4 py-2">
			<div class="flex items-center gap-2">
				<div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
					<span class="text-lg font-bold text-primary-foreground">A</span>
				</div>
				<h1 class="text-lg font-semibold">AutoCorrect</h1>
			</div>

			<nav class="flex gap-1">
				<Button
					onclick={() => (currentTab = 'spellchecker')}
					variant={currentTab === 'spellchecker' ? 'default' : 'ghost'}
					size="sm"
				>
					<Home class="mr-1 h-4 w-4" />
					{tr('app.tab.spellchecker')}
				</Button>
				<Button
					onclick={() => (currentTab = 'settings')}
					variant={currentTab === 'settings' ? 'default' : 'ghost'}
					size="sm"
				>
					<Settings class="mr-1 h-4 w-4" />
					{tr('app.tab.settings')}
				</Button>
				<Button
					onclick={() => (currentTab = 'about')}
					variant={currentTab === 'about' ? 'default' : 'ghost'}
					size="sm"
				>
					<Info class="mr-1 h-4 w-4" />
					{tr('app.tab.about')}
				</Button>
			</nav>

			<StatusIndicator
				bind:isEnabled
				bind:correctionCount
				onToggle={handleToggleEnabled}
				compact={true}
			/>
		</div>
	</header>

	<!-- Main Content Area -->
	<main class="flex-1 overflow-auto">
		{#if currentTab === 'spellchecker'}
			<SpellChecker />
		{:else if currentTab === 'settings'}
			<SettingsPanel {theme} onThemeChange={(mode) => (theme = mode)} />
		{:else if currentTab === 'about'}
			<div class="flex h-full items-center justify-center p-6">
				<div class="max-w-md space-y-4 text-center">
					<div class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-primary">
						<span class="text-3xl font-bold text-primary-foreground">A</span>
					</div>
					<h2 class="text-2xl font-bold">{tr('app.about.title')}</h2>
					<p class="text-muted-foreground">{tr('app.about.desc')}</p>
					<div class="space-y-2 rounded-lg border bg-card p-4 text-left">
						<h3 class="text-sm font-semibold">{tr('app.about.features')}</h3>
						<ul class="space-y-1 text-sm text-muted-foreground">
							<li>{tr('app.about.f1')}</li>
							<li>{tr('app.about.f2')}</li>
							<li>{tr('app.about.f3')}</li>
							<li>{tr('app.about.f4')}</li>
							<li>{tr('app.about.f5')}</li>
						</ul>
					</div>
					<div class="text-xs text-muted-foreground">{tr('app.about.version')}</div>
				</div>
			</div>
		{/if}
	</main>

</div>
