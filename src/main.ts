import { createApp } from "vue";
import GlassView from "./views/GlassView.vue";
import { useTheme } from "./composables/useTheme";

import "@winui/styles/theme.css";
import "@winui/styles/animations.css";

window.addEventListener("contextmenu", (e) => e.preventDefault());

const DIAG = {
  t0: performance.now(),
  tauriOk: false,
  webview2Version: "unknown",
  errors: [] as string[],
};

function diagLog(tag: string, detail?: unknown) {
  const line = `[diag ${((performance.now() - DIAG.t0) / 1000).toFixed(2)}s] ${tag}${detail !== undefined ? " " + String(detail) : ""}`;
  console.log(line);
  DIAG.errors.push(line);
}

async function probeWebview2() {
  try {
    const resp = await fetch("chrome://version");
    const text = await resp.text();
    const m = text.match(/ProductVersion\s*([^;\n]+)/);
    if (m) DIAG.webview2Version = m[1].trim();
  } catch {
    DIAG.webview2Version = "fetch-failed";
  }
  diagLog("webview2", DIAG.webview2Version);
}

window.addEventListener("error", (e) => {
  const msg = `[diag] ERROR: ${e.message} @ ${e.filename}:${e.lineno}`;
  console.error(msg);
  DIAG.errors.push(msg + "\n" + (e.error?.stack || ""));
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Error: " + e.message + "\n\n" + (e.error?.stack || "");
    err.style.display = "block";
  }
});

window.addEventListener("unhandledrejection", (e) => {
  const msg = `[diag] UNHANDLED: ${e.reason}`;
  console.error(msg);
  DIAG.errors.push(msg);
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Unhandled: " + String(e.reason);
    err.style.display = "block";
  }
});

async function currentLabel(): Promise<string> {
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    DIAG.tauriOk = true;
    diagLog("tauri-internals", "detected");
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const label = getCurrentWindow().label;
      diagLog("label", label);
      return label;
    } catch (e) {
      diagLog("getCurrentWindow-failed", e);
      return "";
    }
  }
  diagLog("tauri-internals", "NOT DETECTED");
  return "";
}

async function boot() {
  diagLog("boot-start");
  useTheme();
  probeWebview2();

  const isGlass = (await currentLabel()) === "glass";
  diagLog("mounting", isGlass ? "GlassView" : "App");

  const app = createApp(isGlass ? GlassView : (await import("./App.vue")).default);
  if (!isGlass) {
    const { router } = await import("./router");
    app.use(router);
  }
  app.config.errorHandler = (err, _vm, info) => {
    const msg = `[diag] VUE-ERROR: ${err} ${info}`;
    console.error(msg);
    DIAG.errors.push(msg);
    const el = document.getElementById("app-error");
    if (el) {
      el.textContent = "Vue Error: " + (err as Error).message + "\n\n" + info;
      el.style.display = "block";
    }
  };
  app.mount("#app");
  diagLog("mount-done");
}

try {
  boot();
} catch (e) {
  const msg = `[diag] FATAL: ${e}`;
  console.error(msg);
  DIAG.errors.push(msg + "\n" + (e as Error).stack);
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Fatal: " + (e as Error).message + "\n\n" + (e as Error).stack;
    err.style.display = "block";
  }
}

(window as any).__TOOLSPLUS_DIAG__ = DIAG;

window.addEventListener("keydown", (e) => {
  if (e.key === "F12" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    const win = window as any;
    if (win.__TAURI_INTERNALS__) {
      import("@tauri-apps/api/webview").then(({ getCurrentWebview }) =>
        (getCurrentWebview() as any).openDevTools(),
      );
    }
  }
});