<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Switch } from '$lib/components/ui/switch';
	import { Power, CheckCircle2, XCircle, TrendingUp } from 'lucide-svelte';

	export let isEnabled = true;
	export let correctionCount = 0;
	export let onToggle: ((enabled: boolean) => void) | undefined = undefined;
	export let compact = false;

	function handleToggle() {
		isEnabled = !isEnabled;
		onToggle?.(isEnabled);
	}

	// Format large numbers
	function formatCount(count: number): string {
		if (count >= 1000000) {
			return `${(count / 1000000).toFixed(1)}M`;
		}
		if (count >= 1000) {
			return `${(count / 1000).toFixed(1)}K`;
		}
		return count.toString();
	}

	// Status color
	let statusColor = isEnabled ? 'text-green-600' : 'text-muted-foreground';
	let statusBgColor = isEnabled ? 'bg-green-600/10' : 'bg-muted';
</script>

{#if compact}
	<!-- Compact version suitable for system tray or minimal UI -->
	<div class="flex items-center gap-2">
		<div class={statusBgColor + ' rounded-full p-1'}>
			{#if isEnabled}
				<CheckCircle2 class="h-4 w-4 text-green-600" />
			{:else}
				<XCircle class="h-4 w-4 text-muted-foreground" />
			{/if}
		</div>
		<span class="text-xs font-medium {statusColor}">
			{isEnabled ? 'Active' : 'Disabled'}
		</span>
		{#if correctionCount > 0}
			<span class="text-xs text-muted-foreground">({formatCount(correctionCount)})</span>
		{/if}
		<Switch
			bind:checked={isEnabled}
			onchange={handleToggle}
			className="ml-1"
			title="Toggle AutoCorrect"
		/>
	</div>
{:else}
	<!-- Full version with detailed status -->
	<Card>
		<CardContent class="p-4">
			<div class="flex items-center justify-between">
				<!-- Status display -->
				<div class="flex items-center gap-3">
					<div class={statusBgColor + ' rounded-full p-2'}>
						{#if isEnabled}
							<CheckCircle2 class="h-5 w-5 text-green-600" />
						{:else}
							<XCircle class="h-5 w-5 text-muted-foreground" />
						{/if}
					</div>
					<div>
						<p class="text-sm font-semibold {statusColor}">
							{isEnabled ? 'AutoCorrect Active' : 'AutoCorrect Disabled'}
						</p>
						{#if isEnabled}
							<p class="text-xs text-muted-foreground">Monitoring text for corrections</p>
						{:else}
							<p class="text-xs text-muted-foreground">Paused - toggle to enable</p>
						{/if}
					</div>
				</div>

				<!-- Toggle and stats -->
				<div class="flex items-center gap-4">
					{#if correctionCount > 0}
						<div class="text-right">
							<p class="text-xs text-muted-foreground">Corrections</p>
							<p class="text-lg font-semibold text-primary">
								{formatCount(correctionCount)}
							</p>
						</div>
						<div class="rounded-full bg-primary/10 p-2">
							<TrendingUp class="h-4 w-4 text-primary" />
						</div>
					{/if}

					<div class="flex flex-col items-center gap-1">
						<Switch
							bind:checked={isEnabled}
							onchange={handleToggle}
							title={isEnabled ? 'Disable AutoCorrect' : 'Enable AutoCorrect'}
						/>
						<span class="text-[10px] text-muted-foreground">
							{isEnabled ? 'On' : 'Off'}
						</span>
					</div>

					{#if !compact}
						<Button
							variant={isEnabled ? 'outline' : 'default'}
							size="sm"
							onclick={handleToggle}
							class="gap-1"
						>
							<Power class="h-3 w-3" />
							{isEnabled ? 'Disable' : 'Enable'}
						</Button>
					{/if}
				</div>
			</div>
		</CardContent>
	</Card>
{/if}
