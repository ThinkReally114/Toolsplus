import { ref, watch } from "vue";
import { applyBodyOpacity, getOpacity } from "./useBackdrop";

export type ThemeMode = "light" | "dark" | "system";

const STORAGE_KEY = "toolsplus-theme";
const mode = ref<ThemeMode>(loadMode());

function loadMode(): ThemeMode {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved === "light" || saved === "dark" || saved === "system") return saved;
  return "system";
}

function systemPrefersDark(): boolean {
  return (
    window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false
  );
}

function applyHtmlThemeClass(resolved: "light" | "dark") {
  const html = document.documentElement;
  html.classList.remove("theme-light", "theme-dark");
  html.classList.add(`theme-${resolved}`);
  html.setAttribute("data-theme", resolved);
  applyBodyOpacity(getOpacity());
}

export function resolvedTheme(): "light" | "dark" {
  if (mode.value === "system") return systemPrefersDark() ? "dark" : "light";
  return mode.value;
}

export function setTheme(next: ThemeMode) {
  mode.value = next;
  localStorage.setItem(STORAGE_KEY, next);
}

export function useTheme() {
  const apply = () => applyHtmlThemeClass(resolvedTheme());

  // 监听系统主题变化（仅 system 模式生效）
  if (window.matchMedia) {
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        if (mode.value === "system") applyHtmlThemeClass(resolvedTheme());
      });
  }

  watch(mode, apply, { immediate: true });
  return { mode, setTheme, apply };
}
