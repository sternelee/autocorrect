/**
 * AI feature capability gating.
 *
 * Apple Translation and Local Marian / NLLB backends are *translation-only*
 * neural machine translation systems. They cannot perform grammar
 * polishing, paraphrasing, summarise, clarity analysis, tone detection,
 * or vocabulary enhancement — those tasks require a generative LLM.
 *
 * The full LLM-powered AI tools therefore require an OpenAI-compatible
 * endpoint + API key, regardless of the configured translation provider.
 *
 * This helper centralises that rule so SpellChecker, AiPopup and the
 * Settings panel can consistently disable unsupported buttons and
 * surface a tooltip explaining why.
 */

export type TranslationProviderId = "openai" | "apple" | "local";

export interface AiCapabilities {
  // LLM-driven AI features available in the SpellChecker tab.
  grammar: boolean;
  translate: boolean;
  polish: boolean;
  simplify: boolean;
  clarity: boolean;
  vocabulary: boolean;

  // LLM-driven AI features available in the AiPopup window.
  paraphrase: boolean;
  rewrite: boolean;
  tone: boolean;

  // The dedicated translation pipeline (Settings → Test Translation).
  // Always supported — the `translate_text` command routes to apple,
  // local or openai/dedicated automatically.
  dedicatedTranslation: boolean;
}

export type CapabilityKey = keyof AiCapabilities;

export function getAiCapabilities(
  provider: TranslationProviderId,
  openaiApiKey: string,
): AiCapabilities {
  const llmReady = openaiApiKey.trim().length > 0;
  const isNmtOnly = provider === "apple" || provider === "local";

  // LLM-driven features require both an API key AND a config that isn't
  // set to a translation-only NMT provider.
  const llm = llmReady && !isNmtOnly;

  return {
    grammar: llm,
    translate: llm,
    polish: llm,
    simplify: llm,
    clarity: llm,
    vocabulary: llm,
    paraphrase: llm,
    rewrite: llm,
    tone: llm,
    dedicatedTranslation: true,
  };
}

export function providerLabel(p: TranslationProviderId): string {
  return p === "apple"
    ? "Apple Translation"
    : p === "local"
      ? "Local Marian / NLLB"
      : "OpenAI / OpenRouter";
}

export function isTranslationOnlyProvider(
  provider: TranslationProviderId,
): boolean {
  return provider === "apple" || provider === "local";
}

/**
 * Whether a capability should be **hidden** entirely from the UI.
 *
 * Apple / Local providers cannot ever provide these LLM-driven features,
 * so showing a greyed-out button with a tooltip adds visual noise without
 * surfacing anything actionable. We hide them and rely on the settings
 * panel note for context.
 */
export function isCapHidden(
  capability: CapabilityKey,
  provider: TranslationProviderId,
): boolean {
  if (capability === "dedicatedTranslation") return false;
  return isTranslationOnlyProvider(provider);
}

/**
 * Whether a capability should be rendered **disabled** (still visible).
 *
 * Only fires when the provider supports the capability (i.e. provider is
 * not translation-only) but the prerequisite runtime config — currently
 * just the OpenAI API key — is missing.
 */
export function isCapDisabled(
  capability: CapabilityKey,
  provider: TranslationProviderId,
  openaiApiKey: string,
): boolean {
  if (capability === "dedicatedTranslation") return false;
  if (isCapHidden(capability, provider)) return false;
  return openaiApiKey.trim().length === 0;
}

/**
 * Returns a human-readable reason why a capability is unavailable,
 * or `null` when it is enabled. Pass the result directly to a button's
 * `title` attribute so hovering on a disabled button explains itself.
 */
export function disabledReason(
  capability: CapabilityKey,
  provider: TranslationProviderId,
  openaiApiKey: string,
): string | null {
  if (capability === "dedicatedTranslation") return null;
  const caps = getAiCapabilities(provider, openaiApiKey);
  if (caps[capability]) return null;

  if (!openaiApiKey.trim()) {
    if (isTranslationOnlyProvider(provider)) {
      return `${providerLabel(provider)} is translation-only; OpenAI API key is not configured. Set a key under Settings → AI to enable this feature.`;
    }
    return "OpenAI API key is not configured. Add one under Settings → AI to enable this feature.";
  }
  return `${providerLabel(provider)} is translation-only. This AI feature requires an OpenAI-compatible LLM.`;
}
