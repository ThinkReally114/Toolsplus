import { createApp } from "vue";
import App from "./App.vue";

window.addEventListener("error", (e) => {
  console.error("Window error:", e.message);
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Error: " + e.message + "\n\n" + (e.error?.stack || "");
    err.style.display = "block";
  }
});

window.addEventListener("unhandledrejection", (e) => {
  console.error("Unhandled rejection:", e.reason);
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Unhandled: " + String(e.reason);
    err.style.display = "block";
  }
});

try {
  const app = createApp(App);
  app.config.errorHandler = (err, _vm, info) => {
    console.error("Vue error:", err, info);
    const el = document.getElementById("app-error");
    if (el) {
      el.textContent = "Vue Error: " + (err as Error).message + "\n\n" + info;
      el.style.display = "block";
    }
  };
  app.mount("#app");
  console.log("Vue app mounted!");
} catch (e) {
  console.error("Fatal error:", e);
  const err = document.getElementById("app-error");
  if (err) {
    err.textContent = "Fatal: " + (e as Error).message + "\n\n" + (e as Error).stack;
    err.style.display = "block";
  }
}