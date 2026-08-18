import { invoke } from "@tauri-apps/api/core";

export type BackdropType = "none" | "mica" | "acrylic";

const BACKDROP_KEY = "toolsplus-backdrop";
const OPACITY_KEY = "toolsplus-opacity";

const backdropMap: Record<BackdropType, number> = {
  none: 1,
  mica: 2,
  acrylic: 3,
};

export function getBackdrop(): BackdropType {
  const v = localStorage.getItem(BACKDROP_KEY);
  if (v === "none" || v === "mica" || v === "acrylic") return v;
  return "none";
}

export function getOpacity(): number {
  const v = Number(localStorage.getItem(OPACITY_KEY));
  if (isNaN(v)) return 100;
  return Math.min(100, Math.max(20, v));
}

export function applyBodyOpacity(opacity: number) {
  const alpha = opacity / 100;
  const isDark = document.documentElement.classList.contains("theme-dark");
  const bg = isDark
    ? `rgba(32, 32, 32, ${alpha})`
    : `rgba(243, 243, 243, ${alpha})`;
  const html = document.documentElement;
  const app = document.getElementById("app");
  html.style.setProperty("--app-bg", bg);
  html.style.backgroundColor = bg;
  document.body.style.setProperty("--app-bg", bg);
  document.body.style.backgroundColor = bg;
  if (app) {
    app.style.setProperty("--app-bg", bg);
    app.style.backgroundColor = bg;
  }
}

export async function applyBackdrop(backdrop: BackdropType, opacity: number) {
  localStorage.setItem(BACKDROP_KEY, backdrop);
  localStorage.setItem(OPACITY_KEY, String(opacity));
  applyBodyOpacity(opacity);
  try {
    await invoke("set_window_backdrop", { backdrop: backdropMap[backdrop] });
  } catch {}
}

export function initBackdrop() {
  applyBodyOpacity(getOpacity());
  applyBackdrop(getBackdrop(), getOpacity());
}
