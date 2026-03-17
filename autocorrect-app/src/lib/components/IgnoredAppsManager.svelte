<script lang="ts">
  $locale;
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Switch } from "$lib/components/ui/switch";
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
  import { locale, t } from "$lib/i18n";
  import { Trash2 } from "lucide-svelte";

  interface IgnoredApp {
    name: string;
    bundleId: string;
    ignorePopup: boolean;
    ignoreOverlay: boolean;
  }

  let ignoredApps: IgnoredApp[] = $state([]);
  let loading = $state(false);
  let error = $state("");

  onMount(() => {
    const frameId = requestAnimationFrame(() => {
      void loadIgnoredApps();
    });

    return () => {
      cancelAnimationFrame(frameId);
    };
  });

  async function loadIgnoredApps() {
    loading = true;
    error = "";
    try {
      ignoredApps = await invoke<IgnoredApp[]>("get_ignored_apps");
    } catch (e) {
      error = String(e);
      console.error("Failed to load ignored apps:", e);
    } finally {
      loading = false;
    }
  }

  async function handleUpdatePopup(app: IgnoredApp) {
    try {
      await invoke("update_ignored_app", {
        bundleId: app.bundleId,
        ignorePopup: app.ignorePopup,
        ignoreOverlay: app.ignoreOverlay,
      });
    } catch (e) {
      error = String(e);
      console.error("Failed to update ignored app:", e);
    }
  }

  async function handleUpdateOverlay(app: IgnoredApp) {
    try {
      await invoke("update_ignored_app", {
        bundleId: app.bundleId,
        ignorePopup: app.ignorePopup,
        ignoreOverlay: app.ignoreOverlay,
      });
    } catch (e) {
      error = String(e);
      console.error("Failed to update ignored app:", e);
    }
  }

  async function handleDelete(bundleId: string) {
    try {
      await invoke("remove_ignored_app", { bundleId });
      ignoredApps = ignoredApps.filter((a) => a.bundleId !== bundleId);
    } catch (e) {
      error = String(e);
      console.error("Failed to remove ignored app:", e);
    }
  }
</script>

<div class="card">
  <div class="card-header">
    <h3>{t("settings.ignoredApps")}</h3>
    <p class="card-description">{t("settings.ignoredAppsDesc")}</p>
  </div>

  <div class="card-content">
    {#if loading}
      <div class="loading">{t("common.loading")}</div>
    {:else if error}
      <div class="error">{error}</div>
    {:else if ignoredApps.length === 0}
      <div class="empty-state">{t("settings.ignoredApps.empty")}</div>
    {:else}
      <div class="table-container">
        <table class="table">
          <thead>
            <tr>
              <th>App</th>
              <th>{t("settings.ignoredApps.disablePopup")}</th>
              <th>{t("settings.ignoredApps.disableOverlay")}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each ignoredApps as app (app.bundleId)}
              <tr>
                <td class="app-name">{app.name}</td>
                <td>
                  <Switch
                    bind:checked={app.ignorePopup}
                    onchange={() => handleUpdatePopup(app)}
                  />
                </td>
                <td>
                  <Switch
                    bind:checked={app.ignoreOverlay}
                    onchange={() => handleUpdateOverlay(app)}
                  />
                </td>
                <td>
                  <AlertDialog>
                    <AlertDialogTrigger>
                      <button
                        class="btn-icon btn-danger"
                        title={t("customCorr.delete")}
                      >
                        <Trash2 class="h-4 w-4" />
                      </button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>{t("settings.ignoredApps.deleteTitle")}</AlertDialogTitle>
                        <AlertDialogDescription>
                          {t("settings.ignoredApps.deleteDesc", { name: app.name })}
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
                        <AlertDialogAction
                          onclick={() => handleDelete(app.bundleId)}
                          class="bg-destructive/80 text-white hover:bg-destructive"
                        >
                          {t("common.delete")}
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--card-bg);
    border-radius: 8px;
    border: 1px solid var(--border-color);
    overflow: hidden;
  }

  .card-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-color);
  }

  .card-header h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-color);
  }

  .card-description {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--text-color-secondary);
  }

  .card-content {
    padding: 16px 20px;
  }

  .loading,
  .error,
  .empty-state {
    text-align: center;
    padding: 32px;
    color: var(--text-color-secondary);
    font-size: 14px;
  }

  .error {
    color: var(--danger-color);
  }

  .table-container {
    overflow-x: auto;
  }

  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: 14px;
  }

  .table th,
  .table td {
    padding: 12px;
    text-align: left;
    border-bottom: 1px solid var(--border-color);
  }

  .table th {
    font-weight: 600;
    color: var(--text-color);
    background: var(--bg-secondary);
  }

  .table td {
    color: var(--text-color-secondary);
  }

  .app-name {
    font-weight: 500;
    color: var(--text-color);
  }

  .btn-icon {
    padding: 6px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-color-secondary);
    transition: all 0.2s;
  }

  .btn-icon:hover {
    background: var(--bg-secondary);
    color: var(--text-color);
  }

  .btn-danger:hover {
    background: rgba(255, 59, 48, 0.1);
    color: var(--danger-color);
  }
</style>
