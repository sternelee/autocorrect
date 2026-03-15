import { writable, get } from "svelte/store";
import type { UiLanguage } from "./messages";
import { MESSAGES } from "./messages";

const SUPPORTED_LANGUAGES: UiLanguage[] = ["en", "zh-CN"];

export const locale = writable<UiLanguage>("en");

export function normalizeLanguage(input?: string | null): UiLanguage {
  if (!input) return "en";
  return SUPPORTED_LANGUAGES.includes(input as UiLanguage)
    ? (input as UiLanguage)
    : "en";
}

export function setLocale(lang?: string | null) {
  locale.set(normalizeLanguage(lang));
}

// Translation function
export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  const lang = get(locale);
  let value = MESSAGES[lang][key] ?? MESSAGES.en[key] ?? key;

  if (!params) return value;
  for (const [name, paramValue] of Object.entries(params)) {
    value = value.replaceAll(`{${name}}`, String(paramValue));
  }
  return value;
}

// Reactive translation for use in .svelte templates - use in script with $derived
// Usage: const tr = $derived(new ReactiveT());
export class ReactiveT {
  // This will be accessed reactively via $locale
}
