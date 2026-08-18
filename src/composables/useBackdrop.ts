import { invoke } from "@tauri-apps/api/core";

export type BackdropType = "none" | "mica" | "acrylic";

const BACKDROP_KEY = "toolsplus-backdrop";
const OPACITY_KEY = "toolsplus-opacity";
const KEEP_BLUR_KEY = "toolsplus-keep-blur";

const backdropMap: Record<BackdropType, number> = {
  none: 1,
  mica: 2,
  acrylic: 3,
};

let refreshTimer: number | null = null;
const REFRESH_INTERVAL_MS = 1500;

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

export function getKeepBlur(): boolean {
  return localStorage.getItem(KEEP_BLUR_KEY) === "1";
}

function refreshBackdropOnly() {
  const backdrop = getBackdrop();
  if (backdrop === "none") return;
  invoke("refresh_window_backdrop", { backdrop: backdropMap[backdrop] }).catch(() => {});
}

function startRefreshTimer() {
  if (refreshTimer !== null) return;
  refreshTimer = window.setInterval(() => {
    if (!getKeepBlur()) {
      stopRefreshTimer();
      return;
    }
    refreshBackdropOnly();
  }, REFRESH_INTERVAL_MS);
}

function stopRefreshTimer() {
  if (refreshTimer !== null) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
}

export function setKeepBlur(v: boolean) {
  localStorage.setItem(KEEP_BLUR_KEY, v ? "1" : "0");
  if (v) {
    document.body.classList.remove("window-blurred");
    applyBodyOpacity(getOpacity());
    startRefreshTimer();
  } else {
    stopRefreshTimer();
  }
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
  if (isDark) {
    html.style.setProperty("--dialog-content-bg", "rgb(43, 43, 43)");
    html.style.setProperty("--flyout-bg", "rgb(44, 44, 44)");
    html.style.setProperty("--ToolTipBackgroundBrush", "rgb(40, 40, 40)");
    html.style.setProperty("--TeachingTipBackgroundBrush", "rgb(40, 40, 40)");
    html.style.setProperty("--dialog-background", "rgb(44, 44, 44)");
  } else {
    html.style.setProperty("--dialog-content-bg", "rgb(252, 252, 252)");
    html.style.setProperty("--flyout-bg", "rgb(252, 252, 252)");
    html.style.setProperty("--ToolTipBackgroundBrush", "rgb(252, 252, 252)");
    html.style.setProperty("--TeachingTipBackgroundBrush", "rgb(252, 252, 252)");
    html.style.setProperty("--dialog-background", "rgb(252, 252, 252)");
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
  if (getKeepBlur()) {
    document.body.classList.remove("window-blurred");
    applyBodyOpacity(getOpacity());
    startRefreshTimer();
  }
  initWindowFocusListener();
}

async function initWindowFocusListener() {
  if (typeof (window as any).__TAURI_INTERNALS__ === "undefined") return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    const update = (focused: boolean) => {
      if (getKeepBlur()) {
        document.body.classList.remove("window-blurred");
        applyBodyOpacity(getOpacity());
        startRefreshTimer();
        return;
      }
      stopRefreshTimer();
      if (focused) {
        document.body.classList.remove("window-blurred");
        const op = getOpacity();
        applyBodyOpacity(op);
      } else {
        document.body.classList.add("window-blurred");
        const isDark = document.documentElement.classList.contains("theme-dark");
        const bg = isDark ? "rgb(32, 32, 32)" : "rgb(243, 243, 243)";
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
    };
    const unlisten = await win.onFocusChanged(({ payload: focused }) => {
      update(focused);
    });
    update(await win.isFocused());
    (window as any).__unlistenFocus__ = unlisten;
  } catch (e) {
    console.error(e);
  }
}
