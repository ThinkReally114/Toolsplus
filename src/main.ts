import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { useTheme } from "./composables/useTheme";

import "@winui/styles/theme.css";
import "@winui/styles/animations.css";

window.addEventListener("contextmenu", (e) => e.preventDefault());

window.addEventListener("error", (e) => {
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Error: " + e.message + "\n\n" + (e.error?.stack || "");
    err.style.display = "block";
  }
});

window.addEventListener("unhandledrejection", (e) => {
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Unhandled: " + String(e.reason);
    err.style.display = "block";
  }
});

try {
  useTheme();

  const app = createApp(App);
  app.use(router);
  app.config.errorHandler = (err, _vm, info) => {
    console.error("Vue error:", err, info);
    const el = document.getElementById("app-error");
    if (el) {
      el.textContent = "Vue Error: " + (err as Error).message + "\n\n" + info;
      el.style.display = "block";
    }
  };
  app.mount("#app");
} catch (e) {
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Fatal: " + (e as Error).message + "\n\n" + (e as Error).stack;
    err.style.display = "block";
  }
}