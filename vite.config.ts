import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// WinUIonWeb 子模块根目录（指向其内部 WinUIonWeb/src）
const winuiSrc = fileURLToPath(
  new URL("./external/WinUIonWeb/WinUIonWeb/src", import.meta.url)
);

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],

  base: "./",

  resolve: {
    alias: {
      // 让应用代码可以用 @winui 直接引用 WinUIonWeb 的组件源码
      "@winui": winuiSrc,
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
