<script lang="ts">
  $locale;
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
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
  } from "$lib/components/ui/alert-dialog";
  import { AlertCircle, Plus, Trash2, Edit, Save, X } from "lucide-svelte";
  import { locale, t } from "$lib/i18n";

  // Reactive translation helper
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  });

  interface CustomCorrection {
    typo: string;
    correction: string;
  }

  let corrections: CustomCorrection[] = $state([]);
  let isLoading = $state(false);
  let errorMessage: string | null = $state(null);
  let successMessage: string | null = $state(null);

  // Edit state
  let editingIndex: number | null = $state(null);
  let editTypo = $state("");
  let editCorrection = $state("");

  // Add new state
  let showAddForm = $state(false);
  let newTypo = $state("");
  let newCorrection = $state("");

  // Load corrections on mount
  async function loadCorrections() {
    isLoading = true;
    errorMessage = null;
    try {
      corrections = await invoke<CustomCorrection[]>("get_custom_corrections");
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : tr("customCorr.loadError");
      console.error("Failed to load corrections:", error);
    } finally {
      isLoading = false;
    }
  }

  async function addCorrection() {
    if (!newTypo.trim() || !newCorrection.trim()) {
      errorMessage = tr("customCorr.required");
      return;
    }

    isLoading = true;
    errorMessage = null;
    successMessage = null;

    try {
      await invoke("add_custom_correction", {
        typo: newTypo.trim(),
        correction: newCorrection.trim(),
      });
      successMessage = tr("customCorr.added", { typo: newTypo, correction: newCorrection });
      newTypo = "";
      newCorrection = "";
      showAddForm = false;
      await loadCorrections();

      // Clear success message after 3 seconds
      setTimeout(() => {
        successMessage = null;
      }, 3000);
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : tr("customCorr.addError");
      console.error("Failed to add correction:", error);
    } finally {
      isLoading = false;
    }
  }

  async function deleteCorrection(typo: string) {
    isLoading = true;
    errorMessage = null;
    successMessage = null;

    try {
      await invoke("delete_custom_correction", { typo });
      successMessage = tr("customCorr.deleted", { typo });
      await loadCorrections();

      setTimeout(() => {
        successMessage = null;
      }, 3000);
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : tr("customCorr.deleteError");
      console.error("Failed to delete correction:", error);
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
    editTypo = "";
    editCorrection = "";
  }

  async function saveEdit() {
    if (editingIndex === null) return;

    const oldTypo = corrections[editingIndex].typo;

    if (!editTypo.trim() || !editCorrection.trim()) {
      errorMessage = tr("customCorr.required");
      return;
    }

    isLoading = true;
    errorMessage = null;
    successMessage = null;

    try {
      await invoke("update_custom_correction", {
        oldTypo,
        newTypo: editTypo.trim(),
        newCorrection: editCorrection.trim(),
      });
      successMessage = tr("customCorr.updated", { typo: editTypo, correction: editCorrection });
      editingIndex = null;
      editTypo = "";
      editCorrection = "";
      await loadCorrections();

      // Clear success message after 3 seconds
      setTimeout(() => {
        successMessage = null;
      }, 3000);
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : tr("customCorr.updateError");
      console.error("Failed to update correction:", error);
    } finally {
      isLoading = false;
    }
  }

  function cancelAdd() {
    showAddForm = false;
    newTypo = "";
    newCorrection = "";
  }

  // Load on mount
  loadCorrections();
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <div>
      <h3 class="text-sm font-semibold">{tr('customCorr.title')}</h3>
      <p class="text-xs text-muted-foreground mt-1">
        {tr('customCorr.desc')}
      </p>
    </div>
    <Button
      onclick={() => (showAddForm = true)}
      variant="default"
      size="sm"
      disabled={isLoading || showAddForm}
    >
      <Plus class="mr-2 h-4 w-4" />
      {tr('customCorr.addNew')}
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
        <AlertCircle
          class="h-4 w-4 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5"
        />
        <div class="flex-1">
          <p class="text-sm text-red-900 dark:text-red-100">{errorMessage}</p>
        </div>
        <button
          onclick={() => (errorMessage = null)}
          class="text-red-600 dark:text-red-400"
        >
          &times;
        </button>
      </div>
    </div>
  {/if}

  <!-- Add New Form -->
  {#if showAddForm}
    <div class="rounded-lg border p-4 bg-muted/50">
      <h4 class="text-sm font-medium mb-3">{tr('customCorr.addTitle')}</h4>
      <div class="space-y-3">
        <div>
          <label
            for="new-typo"
            class="text-xs font-medium text-muted-foreground">{tr('customCorr.typo')}</label
          >
          <Input
            id="new-typo"
            bind:value={newTypo}
            placeholder="e.g., whts"
            class="font-mono text-sm mt-1"
            disabled={isLoading}
          />
        </div>
        <div>
          <label
            for="new-correction"
            class="text-xs font-medium text-muted-foreground">{tr('customCorr.correction')}</label
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
          <Button
            onclick={addCorrection}
            variant="default"
            size="sm"
            disabled={isLoading}
          >
            <Save class="mr-2 h-3 w-3" />
            {tr('customCorr.save')}
          </Button>
          <Button
            onclick={cancelAdd}
            variant="outline"
            size="sm"
            disabled={isLoading}
          >
            <X class="mr-2 h-3 w-3" />
            {tr('customCorr.cancel')}
          </Button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Corrections List -->
  {#if isLoading && corrections.length === 0}
    <div class="flex items-center justify-center p-8">
      <div
        class="h-6 w-6 animate-spin rounded-full border-2 border-current border-t-transparent"
      ></div>
    </div>
  {:else if corrections.length === 0}
    <div class="rounded-lg border p-8 text-center">
      <p class="text-sm text-muted-foreground">
        {tr('customCorr.empty')}
      </p>
      <p class="text-xs text-muted-foreground mt-2">
        {tr('customCorr.example')}
      </p>
    </div>
  {:else}
    <div class="rounded-lg border overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-muted">
          <tr>
            <th class="px-4 py-2 text-left font-medium">{tr('customCorr.typo')}</th>
            <th class="px-4 py-2 text-left font-medium">{tr('customCorr.correction')}</th>
            <th class="px-4 py-2 text-right font-medium">{tr('customCorr.actions')}</th>
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
                      title={tr('customCorr.save')}
                    >
                      <Save class="h-4 w-4" />
                    </button>
                    <button
                      onclick={cancelEdit}
                      class="rounded p-1 hover:bg-muted"
                      disabled={isLoading}
                      title={tr('customCorr.cancel')}
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
                      title={tr('customCorr.edit')}
                    >
                      <Edit class="h-4 w-4" />
                    </button>
                    <AlertDialog>
                      <AlertDialogTrigger>
                        <button
                          class="rounded p-1 hover:bg-muted text-red-600 dark:text-red-400"
                          disabled={isLoading}
                          title={tr('customCorr.delete')}
                        >
                          <Trash2 class="h-4 w-4" />
                        </button>
                      </AlertDialogTrigger>
                      <AlertDialogContent>
                        <AlertDialogHeader>
                          <AlertDialogTitle>{tr('customCorr.deleteTitle')}</AlertDialogTitle
                          >
                          <AlertDialogDescription>
                            {tr('customCorr.deleteDesc', { typo: correction.typo, correction: correction.correction })}
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel>{tr('customCorr.cancel')}</AlertDialogCancel>
                          <AlertDialogAction
                            onclick={() => deleteCorrection(correction.typo)}
                            class="bg-destructive/80 text-white hover:bg-destructive"
                          >
                            {tr('customCorr.delete')}
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
      {tr('customCorr.total', { count: corrections.length })}
    </p>
  {/if}
</div>
