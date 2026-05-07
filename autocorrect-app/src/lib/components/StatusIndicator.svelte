<script lang="ts">
  $locale;
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent } from "$lib/components/ui/card";
  import { Switch } from "$lib/components/ui/switch";
  import { Power, CheckCircle2, XCircle, TrendingUp } from "lucide-svelte";
  import { locale, t } from "$lib/i18n";

  interface Props {
    isEnabled?: boolean;
    correctionCount?: number;
    onToggle?: ((enabled: boolean) => void) | undefined;
    compact?: boolean;
  }

  let {
    isEnabled = $bindable(true),
    correctionCount = $bindable(0),
    onToggle = undefined,
    compact = false,
  }: Props = $props();

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
  let statusColor = isEnabled ? "text-green-600" : "text-muted-foreground";
  let statusBgColor = isEnabled ? "bg-green-600/10" : "bg-muted";
</script>

{#if compact}
  <!-- Compact version suitable for system tray or minimal UI -->
  <div class="flex items-center gap-2" data-locale={$locale}>
    <div class={statusBgColor + " rounded-full p-1"}>
      {#if isEnabled}
        <CheckCircle2 class="h-4 w-4 text-green-600" />
      {:else}
        <XCircle class="h-4 w-4 text-muted-foreground" />
      {/if}
    </div>
    <span class="text-xs font-medium {statusColor}">
      {isEnabled ? t("status.active") : t("status.disabled")}
    </span>
    {#if correctionCount > 0}
      <span class="text-xs text-muted-foreground"
        >({formatCount(correctionCount)})</span
      >
    {/if}
    <Switch
      bind:checked={isEnabled}
      onchange={handleToggle}
      className="ml-1"
      title={t("status.toggle")}
    />
  </div>
{:else}
  <!-- Full version with detailed status -->
  <Card data-locale={$locale}>
    <CardContent class="p-4">
      <div class="flex items-center justify-between">
        <!-- Status display -->
        <div class="flex items-center gap-3">
          <div class={statusBgColor + " rounded-full p-2"}>
            {#if isEnabled}
              <CheckCircle2 class="h-5 w-5 text-green-600" />
            {:else}
              <XCircle class="h-5 w-5 text-muted-foreground" />
            {/if}
          </div>
          <div>
            <p class="text-sm font-semibold {statusColor}">
              {isEnabled ? t("status.auto_active") : t("status.auto_disabled")}
            </p>
            {#if isEnabled}
              <p class="text-xs text-muted-foreground">
                {t("status.monitoring")}
              </p>
            {:else}
              <p class="text-xs text-muted-foreground">{t("status.paused")}</p>
            {/if}
          </div>
        </div>

        <!-- Toggle and stats -->
        <div class="flex items-center gap-4">
          {#if correctionCount > 0}
            <div class="text-right">
              <p class="text-xs text-muted-foreground">
                {t("status.corrections")}
              </p>
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
              title={isEnabled
                ? t("status.disable_tip")
                : t("status.enable_tip")}
            />
            <span class="text-[10px] text-muted-foreground">
              {isEnabled ? t("status.on") : t("status.off")}
            </span>
          </div>

          {#if !compact}
            <Button
              variant={isEnabled ? "outline" : "default"}
              size="sm"
              onclick={handleToggle}
              class="gap-1"
            >
              <Power class="h-3 w-3" />
              {isEnabled ? t("status.disable") : t("status.enable")}
            </Button>
          {/if}
        </div>
      </div>
    </CardContent>
  </Card>
{/if}
