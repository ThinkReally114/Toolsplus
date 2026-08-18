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
    appShell.style.transformOrigin = "top left";
    appShell.style.transform = `scale(${factor})`;
    appShell.style.width = `${100 / factor}%`;
    appShell.style.height = `${100 / factor}%`;
  }
}

export function initZoom() {
  applyZoom(webviewZoom.value);
}

export function useZoom() {
  return { webviewZoom, applyZoom };
}
