<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import {
		AlertDialog,
		AlertDialogTrigger,
		AlertDialogContent,
		AlertDialogHeader,
		AlertDialogFooter,
		AlertDialogTitle,
		AlertDialogDescription,
		AlertDialogAction,
		AlertDialogCancel,
	} from '$lib/components/ui/alert-dialog';
	import { AlertCircle, Plus, Trash2, Edit, Save, X } from 'lucide-svelte';

	interface CustomCorrection {
		typo: string;
		correction: string;
	}

	let corrections: CustomCorrection[] = [];
	let isLoading = false;
	let errorMessage: string | null = null;
	let successMessage: string | null = null;

	// Edit state
	let editingIndex: number | null = null;
	let editTypo = '';
	let editCorrection = '';


	// Add new state
	let showAddForm = false;
	let newTypo = '';
	let newCorrection = '';

	// Load corrections on mount
	async function loadCorrections() {
		isLoading = true;
		errorMessage = null;
		try {
			corrections = await invoke<CustomCorrection[]>('get_custom_corrections');
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to load corrections';
			console.error('Failed to load corrections:', error);
		} finally {
			isLoading = false;
		}
	}

	async function addCorrection() {
		if (!newTypo.trim() || !newCorrection.trim()) {
			errorMessage = 'Both typo and correction are required';
			return;
		}

		isLoading = true;
		errorMessage = null;
		successMessage = null;

		try {
			await invoke('add_custom_correction', {
				typo: newTypo.trim(),
				correction: newCorrection.trim()
			});
			successMessage = `Added: ${newTypo} → ${newCorrection}`;
			newTypo = '';
			newCorrection = '';
			showAddForm = false;
			await loadCorrections();

			// Clear success message after 3 seconds
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to add correction';
			console.error('Failed to add correction:', error);
		} finally {
			isLoading = false;
		}
	}

	async function deleteCorrection(typo: string) {
		isLoading = true;
		errorMessage = null;
		successMessage = null;

		try {
			await invoke('delete_custom_correction', { typo });
			successMessage = `Deleted: ${typo}`;
			await loadCorrections();

			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to delete correction';
			console.error('Failed to delete correction:', error);
		} finally {
			isLoading = false;
		}
	}

	function startEdit(index: number) {
		editingIndex = index;
		editTypo = corrections[index].typo;
		editCorrection = corrections[index].correction;
	}

	function cancelEdit() {
		editingIndex = null;
		editTypo = '';
		editCorrection = '';
	}

	async function saveEdit() {
		if (editingIndex === null) return;

		const oldTypo = corrections[editingIndex].typo;

		if (!editTypo.trim() || !editCorrection.trim()) {
			errorMessage = 'Both typo and correction are required';
			return;
		}

		isLoading = true;
		errorMessage = null;
		successMessage = null;

		try {
			await invoke('update_custom_correction', {
				oldTypo,
				newTypo: editTypo.trim(),
				newCorrection: editCorrection.trim()
			});
			successMessage = `Updated: ${editTypo} → ${editCorrection}`;
			editingIndex = null;
			editTypo = '';
			editCorrection = '';
			await loadCorrections();

			// Clear success message after 3 seconds
			setTimeout(() => {
				successMessage = null;
			}, 3000);
		} catch (error) {
			errorMessage = error instanceof Error ? error.message : 'Failed to update correction';
			console.error('Failed to update correction:', error);
		} finally {
			isLoading = false;
		}
	}

	function cancelAdd() {
		showAddForm = false;
		newTypo = '';
		newCorrection = '';
	}

	// Load on mount
	loadCorrections();
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<div>
			<h3 class="text-sm font-semibold">Custom Typo Corrections</h3>
			<p class="text-xs text-muted-foreground mt-1">
				Define your own typo → correction mappings. Changes take effect immediately.
			</p>
		</div>
		<Button
			onclick={() => (showAddForm = true)}
			variant="default"
			size="sm"
			disabled={isLoading || showAddForm}
		>
			<Plus class="mr-2 h-4 w-4" />
			Add New
		</Button>
	</div>

	<!-- Success Message -->
	{#if successMessage}
		<div
			class="rounded-lg border border-green-200 bg-green-50 p-3 dark:border-green-800 dark:bg-green-950"
		>
			<p class="text-sm text-green-900 dark:text-green-100">{successMessage}</p>
		</div>
	{/if}

	<!-- Error Message -->
	{#if errorMessage}
		<div
			class="rounded-lg border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-950"
		>
			<div class="flex items-start gap-2">
				<AlertCircle class="h-4 w-4 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
				<div class="flex-1">
					<p class="text-sm text-red-900 dark:text-red-100">{errorMessage}</p>
				</div>
				<button onclick={() => (errorMessage = null)} class="text-red-600 dark:text-red-400">
					&times;
				</button>
			</div>
		</div>
	{/if}

	<!-- Add New Form -->
	{#if showAddForm}
		<div class="rounded-lg border p-4 bg-muted/50">
			<h4 class="text-sm font-medium mb-3">Add New Correction</h4>
			<div class="space-y-3">
				<div>
					<label for="new-typo" class="text-xs font-medium text-muted-foreground">Typo</label>
					<Input
						id="new-typo"
						bind:value={newTypo}
						placeholder="e.g., whts"
						class="font-mono text-sm mt-1"
						disabled={isLoading}
					/>
				</div>
				<div>
					<label for="new-correction" class="text-xs font-medium text-muted-foreground"
						>Correction</label
					>
					<Input
						id="new-correction"
						bind:value={newCorrection}
						placeholder="e.g., what's"
						class="font-mono text-sm mt-1"
						disabled={isLoading}
					/>
				</div>
				<div class="flex gap-2 pt-2">
					<Button onclick={addCorrection} variant="default" size="sm" disabled={isLoading}>
						<Save class="mr-2 h-3 w-3" />
						Save
					</Button>
					<Button onclick={cancelAdd} variant="outline" size="sm" disabled={isLoading}>
						<X class="mr-2 h-3 w-3" />
						Cancel
					</Button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Corrections List -->
	{#if isLoading && corrections.length === 0}
		<div class="flex items-center justify-center p-8">
			<div class="h-6 w-6 animate-spin rounded-full border-2 border-current border-t-transparent"></div>
		</div>
	{:else if corrections.length === 0}
		<div class="rounded-lg border p-8 text-center">
			<p class="text-sm text-muted-foreground">
				No custom corrections yet. Add one above to get started!
			</p>
			<p class="text-xs text-muted-foreground mt-2">
				Example: "whts" → "what's", "teh" → "the"
			</p>
		</div>
	{:else}
		<div class="rounded-lg border overflow-hidden">
			<table class="w-full text-sm">
				<thead class="bg-muted">
					<tr>
						<th class="px-4 py-2 text-left font-medium">Typo</th>
						<th class="px-4 py-2 text-left font-medium">Correction</th>
						<th class="px-4 py-2 text-right font-medium">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each corrections as correction, index}
						<tr class="border-t hover:bg-muted/50">
							{#if editingIndex === index}
								<!-- Edit Mode -->
								<td class="px-4 py-2">
									<Input
										bind:value={editTypo}
										class="font-mono text-sm h-8"
										disabled={isLoading}
									/>
								</td>
								<td class="px-4 py-2">
									<Input
										bind:value={editCorrection}
										class="font-mono text-sm h-8"
										disabled={isLoading}
									/>
								</td>
								<td class="px-4 py-2">
									<div class="flex justify-end gap-1">
										<button
											onclick={saveEdit}
											class="rounded p-1 hover:bg-muted text-green-600 dark:text-green-400"
											disabled={isLoading}
											title="Save"
										>
											<Save class="h-4 w-4" />
										</button>
										<button
											onclick={cancelEdit}
											class="rounded p-1 hover:bg-muted"
											disabled={isLoading}
											title="Cancel"
										>
											<X class="h-4 w-4" />
										</button>
									</div>
								</td>
							{:else}
								<!-- View Mode -->
								<td class="px-4 py-2 font-mono">{correction.typo}</td>
								<td class="px-4 py-2 font-mono">{correction.correction}</td>
								<td class="px-4 py-2">
									<div class="flex justify-end gap-1">
										<button
											onclick={() => startEdit(index)}
											class="rounded p-1 hover:bg-muted"
											disabled={isLoading}
											title="Edit"
										>
											<Edit class="h-4 w-4" />
										</button>
										<AlertDialog>
											<AlertDialogTrigger>
												<button
													class="rounded p-1 hover:bg-muted text-red-600 dark:text-red-400"
													disabled={isLoading}
													title="Delete"
												>
													<Trash2 class="h-4 w-4" />
												</button>
											</AlertDialogTrigger>
											<AlertDialogContent>
												<AlertDialogHeader>
													<AlertDialogTitle>Delete correction?</AlertDialogTitle>
													<AlertDialogDescription>
														Remove <span class="font-mono font-medium">"{correction.typo}"</span>
														→ <span class="font-mono font-medium">"{correction.correction}"</span>
														from custom corrections. This cannot be undone.
													</AlertDialogDescription>
												</AlertDialogHeader>
												<AlertDialogFooter>
													<AlertDialogCancel>Cancel</AlertDialogCancel>
													<AlertDialogAction
														onclick={() => deleteCorrection(correction.typo)}
														class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
													>
														Delete
													</AlertDialogAction>
												</AlertDialogFooter>
											</AlertDialogContent>
										</AlertDialog>
									</div>
								</td>
							{/if}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<p class="text-xs text-muted-foreground">
			Total: {corrections.length} custom {corrections.length === 1 ? 'correction' : 'corrections'}
		</p>
	{/if}
</div>
