import type { ThemeMode } from "$lib/types/theme";

export const THEME_STORAGE_KEY = "autocorrect-theme";

export function isThemeMode(value: string): value is ThemeMode {
  return value === "light" || value === "dark" || value === "auto";
}

export function loadThemeFromLocalStorage(): ThemeMode {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored && isThemeMode(stored)) {
    return stored;
  }
  return "auto";
}

export function saveThemeToLocalStorage(mode: ThemeMode) {
  localStorage.setItem(THEME_STORAGE_KEY, mode);
}

export function applyThemeToDom(mode: ThemeMode) {
  const html = document.documentElement;
  if (mode === "dark") {
    html.classList.add("dark");
  } else if (mode === "auto") {
    const prefersDark = window.matchMedia(
      "(prefers-color-scheme: dark)",
    ).matches;
    html.classList.toggle("dark", prefersDark);
  } else {
    html.classList.remove("dark");
  }
}
