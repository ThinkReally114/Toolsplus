import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { useTheme } from "./composables/useTheme";

// 引入 WinUIonWeb 的主题样式（含亮/暗主题变量、动画）
import "@winui/styles/theme.css";
import "@winui/styles/animations.css";

// 初始化主题（必须在 App 挂载前应用 html 主题类）
useTheme();

createApp(App).use(router).mount("#app");
