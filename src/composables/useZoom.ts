import { ref } from "vue";

const ZOOM_KEY = "toolsplus-webview-zoom";

function loadZoom(): number {
  const v = Number(localStorage.getItem(ZOOM_KEY));
  if (isNaN(v) || v === 0) return 100;
  return Math.min(200, Math.max(50, v));
}

export const webviewZoom = ref<number>(loadZoom());

export function getZoom(): number {
  return webviewZoom.value;
}

export function applyZoom(zoom: number) {
  const safe = Math.min(200, Math.max(50, zoom || 100));
  webviewZoom.value = safe;
  localStorage.setItem(ZOOM_KEY, String(safe));
  const factor = safe / 100;
  document.documentElement.style.setProperty("--app-zoom", String(factor));
  const appShell = document.querySelector<HTMLElement>(".app-shell");
  if (appShell) {
    appShell.style.zoom = String(factor);
    appShell.style.transform = "";
    appShell.style.width = "";
    appShell.style.height = "";
  }
}

export function initZoom() {
  applyZoom(webviewZoom.value);
}

export function useZoom() {
  return { webviewZoom, applyZoom };
}
