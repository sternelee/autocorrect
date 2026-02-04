<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Card, CardContent } from '$lib/components/ui/card';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Check, X, Edit2 } from 'lucide-svelte';

	export let show = false;
	export let originalText = '';
	export let suggestion = '';
	export let position = { x: 0, y: 0 };
	export let onAccept: ((text: string) => void) | undefined = undefined;
	export let onReject: (() => void) | undefined = undefined;

	let isEditing = false;
	let customSuggestion = '';

	function handleAccept() {
		const textToUse = isEditing ? customSuggestion : suggestion;
		onAccept?.(textToUse);
		closePopup();
	}

	function handleReject() {
		onReject?.();
		closePopup();
	}

	function closePopup() {
		show = false;
		isEditing = false;
		customSuggestion = '';
	}

	function startEditing() {
		isEditing = true;
		customSuggestion = suggestion;
	}

	function cancelEdit() {
		isEditing = false;
		customSuggestion = '';
	}

	// Keyboard shortcuts
	function handleKeydown(e: KeyboardEvent) {
		if (!show) return;

		if (e.key === 'Enter' && !e.shiftKey && !isEditing) {
			e.preventDefault();
			handleAccept();
		} else if (e.key === 'Escape') {
			e.preventDefault();
			if (isEditing) {
				cancelEdit();
			} else {
				handleReject();
			}
		}
	}

	// Calculate position to keep popup in viewport
	let computedStyle = `left: ${Math.min(position.x, window.innerWidth - 350)}px; top: ${Math.min(position.y, window.innerHeight - 200)}px;`;
</script>

<svelte:window onkeydown={handleKeydown} />

{#if show}
	<div
		class="fixed z-50 animate-in fade-in slide-in-from-top-2 duration-200"
		style={computedStyle}
	>
		<Card class="w-[320px] shadow-lg">
			<CardContent class="p-4 space-y-3">
				<!-- Header -->
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-semibold text-foreground">Suggestion</h3>
					<div class="flex gap-1">
						<button
							onclick={handleReject}
							class="rounded-full p-1 hover:bg-muted transition-colors"
							title="Reject (Esc)"
						>
							<X class="h-4 w-4 text-destructive" />
						</button>
					</div>
				</div>

				<!-- Original text -->
				{#if originalText}
					<div class="space-y-1">
						<p class="text-xs text-muted-foreground">Original:</p>
						<p class="text-sm text-destructive line-through">{originalText}</p>
					</div>
				{/if}

				<!-- Suggestion display -->
				{#if isEditing}
					<div class="space-y-1">
						<p class="text-xs text-muted-foreground">Edit correction:</p>
						<Textarea
							bind:value={customSuggestion}
							class="min-h-[60px] text-sm"
							placeholder="Custom correction..."
							autofocus
						/>
					</div>
				{:else}
					<div class="space-y-1">
						<p class="text-xs text-muted-foreground">Suggested:</p>
						<p class="text-sm font-medium text-green-600 dark:text-green-400">{suggestion}</p>
					</div>
				{/if}

				<!-- Action buttons -->
				<div class="flex justify-between gap-2 pt-2">
					{#if isEditing}
						<Button onclick={cancelEdit} variant="outline" size="sm">
							Cancel
						</Button>
						<Button
							onclick={handleAccept}
							variant="default"
							size="sm"
							disabled={!customSuggestion.trim()}
						>
							<Check class="mr-1 h-3 w-3" />
							Use Custom
						</Button>
					{:else}
						<Button onclick={startEditing} variant="outline" size="sm">
							<Edit2 class="mr-1 h-3 w-3" />
							Edit
						</Button>
						<Button onclick={handleAccept} variant="default" size="sm">
							<Check class="mr-1 h-3 w-3" />
							Accept
						</Button>
					{/if}
				</div>

				<!-- Keyboard shortcuts hint -->
				<div class="text-[10px] text-muted-foreground text-center pt-1 border-t">
					Enter=Accept, Esc=Cancel
				</div>
			</CardContent>
		</Card>
	</div>
{/if}
