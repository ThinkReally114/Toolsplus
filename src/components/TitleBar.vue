<script setup lang="ts">
import { ref, onMounted } from "vue";
import AppIcon from "@/components/AppIcon.vue";
import { inject } from "vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;

const isMaximized = ref(false);
const hasTauri = ref(false);

async function checkTauri() {
  try {
    hasTauri.value = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    if (hasTauri.value) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const appWindow = getCurrentWindow();
      isMaximized.value = await appWindow.isMaximized();
    }
  } catch {
    hasTauri.value = false;
  }
}

onMounted(checkTauri);

async function onMinimize() {
  if (!hasTauri.value) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  } catch (e) {
    console.error(e);
  }
}

async function onToggleMaximize() {
  if (!hasTauri.value) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    await appWindow.toggleMaximize();
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) {
    console.error(e);
  }
}

async function onClose() {
  if (!hasTauri.value) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <div class="titlebar" data-tauri-drag-region>
    <div class="titlebar-left" data-tauri-drag-region>
      <AppIcon name="app" :size="16" class="titlebar-logo" />
      <span class="titlebar-title" data-tauri-drag-region>{{ i18n.t("app.title") }}</span>
    </div>
    <div class="titlebar-controls">
      <button class="titlebar-btn" :title="i18n.t('titlebar.minimize')" @click="onMinimize" :disabled="!hasTauri">
        <AppIcon name="minimize" :size="16" />
      </button>
      <button class="titlebar-btn" :title="i18n.t('titlebar.maximize')" @click="onToggleMaximize" :disabled="!hasTauri">
        <AppIcon :name="isMaximized ? 'restore' : 'maximize'" :size="16" />
      </button>
      <button class="titlebar-btn titlebar-close" :title="i18n.t('titlebar.close')" @click="onClose" :disabled="!hasTauri">
        <AppIcon name="close" :size="16" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 16px;
  flex-shrink: 0;
  user-select: none;
  -webkit-app-region: drag;
  background: var(--SolidBackgroundFillColorBaseBrush, #f3f3f3);
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}

html.theme-dark .titlebar {
  background: var(--SolidBackgroundFillColorBaseBrush, #202020);
  border-bottom-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.08));
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 100%;
}

.titlebar-logo {
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .titlebar-logo {
  color: var(--AccentButtonBackground, #4cc2ff);
}

.titlebar-title {
  font-size: 13px;
  font-weight: 600;
  opacity: 0.9;
}

.titlebar-controls {
  display: flex;
  align-items: center;
  height: 100%;
  -webkit-app-region: no-drag;
}

.titlebar-btn {
  width: 46px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: inherit;
  cursor: pointer;
  transition: background 0.12s ease;
  font-family: inherit;
}

.titlebar-btn:hover:not(:disabled) {
  background: var(--SubtleFillColorTertiary, rgba(0, 0, 0, 0.06));
}

html.theme-dark .titlebar-btn:hover:not(:disabled) {
  background: var(--SubtleFillColorTertiary, rgba(255, 255, 255, 0.08));
}

.titlebar-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.titlebar-close:hover:not(:disabled) {
  background: #c42b1c;
  color: #fff;
}
</style>
