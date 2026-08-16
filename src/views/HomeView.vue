<script setup lang="ts">
import { inject, computed, ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinSelectorBar from "@winui/components/WinSelectorBar.vue";
import WinHorizontalScrollContainer from "@winui/components/WinHorizontalScrollContainer.vue";
import WinCase from "@winui/components/WinCase.vue";
import WinSwitchPresenter from "@winui/components/WinSwitchPresenter.vue";
import AppIcon from "@/components/AppIcon.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;
const router = useRouter();

interface DiskInfo {
  name: string;
  model: string;
  total: string;
  free: string;
  disk_type: string;
  interface: string;
}

interface HardwareInfo {
  motherboard: string;
  cpu: string;
  gpu: string;
  ram_total: string;
  ram_used: string;
  ram_speed: string;
  ram_manufacturer: string;
  gpu_vram_total: string;
  gpu_driver: string;
  disks: DiskInfo[];
}

const hw = ref<HardwareInfo | null>(null);
const hwError = ref("");
const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function loadHardware() {
  if (!hasTauri) {
    hwError.value = "demo";
    return;
  }
  try {
    hw.value = await invoke<HardwareInfo>("get_hardware_info");
  } catch (e) {
    hwError.value = String(e);
  }
}

onMounted(loadHardware);

const headerTiles = computed(() => [
  {
    tag: "diskclean",
    icon: "diskclean",
    titleKey: "nav.diskclean",
    descKey: "diskclean.subtitle",
  },
  {
    tag: "performance",
    icon: "performance",
    titleKey: "nav.performance",
    descKey: "performance.subtitle",
  },
  {
    tag: "process",
    icon: "process",
    titleKey: "nav.process",
    descKey: "process.subtitle",
  },
  {
    tag: "optimize",
    icon: "optimize",
    titleKey: "nav.optimize",
    descKey: "optimize.subtitle",
  },
  {
    tag: "git",
    icon: "git",
    titleKey: "nav.git",
    descKey: "git.subtitle",
  },
  {
    tag: "settings",
    icon: "settings",
    titleKey: "nav.settings",
    descKey: "settings.subtitle",
  },
]);

const tools = computed(() => [
  {
    tag: "diskclean",
    icon: "diskclean",
    titleKey: "nav.diskclean",
    descKey: "diskclean.subtitle",
  },
  {
    tag: "performance",
    icon: "performance",
    titleKey: "nav.performance",
    descKey: "performance.subtitle",
  },
  {
    tag: "process",
    icon: "process",
    titleKey: "nav.process",
    descKey: "process.subtitle",
  },
  {
    tag: "git",
    icon: "git",
    titleKey: "nav.git",
    descKey: "git.subtitle",
  },
]);

const hardwareCards = computed(() => {
  const loading = i18n.t("home.hardware.loading");
  const unknown = i18n.t("home.hardware.unknown");
  const h = hw.value;
  return [
    {
      icon: "motherboard",
      label: i18n.t("home.hardware.motherboard"),
      value: hwError.value ? unknown : h?.motherboard || loading,
    },
    {
      icon: "cpu",
      label: i18n.t("home.hardware.cpu"),
      value: hwError.value ? unknown : h?.cpu || loading,
    },
    {
      icon: "gpu",
      label: i18n.t("home.hardware.gpu"),
      value: hwError.value ? unknown : h?.gpu || loading,
    },
    {
      icon: "ram",
      label: i18n.t("home.hardware.ram"),
      value: hwError.value
        ? unknown
        : h
        ? i18n.t("home.hardware.ramUsage", {
            used: h.ram_used,
            total: h.ram_total,
          })
        : loading,
    },
    {
      icon: "ram",
      label: i18n.t("home.hardware.ramSpeed"),
      value: hwError.value ? unknown : h?.ram_speed || loading,
    },
    {
      icon: "ram",
      label: i18n.t("home.hardware.ramBrand"),
      value: hwError.value ? unknown : h?.ram_manufacturer || loading,
    },
    {
      icon: "gpu",
      label: i18n.t("home.hardware.gpuVram"),
      value: hwError.value ? unknown : h?.gpu_vram_total || loading,
    },
    {
      icon: "gpu",
      label: i18n.t("home.hardware.gpuDriver"),
      value: hwError.value ? unknown : h?.gpu_driver || loading,
    },
  ];
});

const selectedFilterIndex = ref(0);
const selectedFilter = ref("tools");

const filterItems = computed(() => [
  { Text: i18n.t("home.tools.title"), Tag: "tools" },
  { Text: i18n.t("home.hardware.title"), Tag: "hardware" },
]);

function onFilterChanged(sender: any) {
  const selectedItem = sender?.SelectedItem;
  const index = Math.max(0, sender?.Items?.indexOf(selectedItem) ?? 0);
  selectedFilterIndex.value = index;
  selectedFilter.value = selectedItem?.Tag ?? "tools";
}

function go(tag: string) {
  router.push({ name: tag });
}
</script>

<template>
  <WinScrollViewer class="gallery-home-scroll" VerticalScrollBarVisibility="Auto" VerticalScrollMode="Auto">
    <div class="gallery-home-page">
      <div class="home-page">
        <section class="home-page-header">
          <div class="home-header-image-mask">
            <div class="home-header-image-grid">
              <div class="home-header-image"></div>
            </div>
          </div>

          <div class="home-header-copy">
            <WinTextBlock
              class="home-header-title"
              :Text="i18n.t('app.title')"
              :FontSize="40"
              FontWeight="600"
              :LineHeight="52"
            />
          </div>

          <WinHorizontalScrollContainer class="home-header-tiles-scroll">
            <div class="home-header-tiles">
              <button
                v-for="tile in headerTiles"
                :key="tile.tag"
                class="win-home-header-tile"
                type="button"
                @click="go(tile.tag)"
              >
                <span class="win-home-header-tile-content">
                  <span class="win-home-header-tile-source">
                    <AppIcon :name="tile.icon" :size="24" />
                  </span>
                  <span class="win-home-header-tile-text">
                    <span class="win-home-header-tile-title">{{ i18n.t(tile.titleKey) }}</span>
                    <span class="win-home-header-tile-description">{{ i18n.t(tile.descKey) }}</span>
                  </span>
                  <span class="win-home-header-tile-open-icon icon" aria-hidden="true">&#xE8A7;</span>
                </span>
              </button>
            </div>
          </WinHorizontalScrollContainer>
        </section>

        <WinSelectorBar
          :class="['filter-bar', 'token-filter-bar', { 'is-cjk-locale': i18n.locale === 'zh-CN' }]"
          HorizontalAlignment="Center"
          :Items="filterItems"
          :SelectedItem="filterItems[selectedFilterIndex]"
          @SelectionChanged="onFilterChanged"
        />

        <WinSwitchPresenter class="switch-presenter" :Value="selectedFilter">
          <WinCase Value="tools">
            <section class="sample-panel">
              <div class="grid-view">
                <button
                  v-for="tool in tools"
                  :key="tool.tag"
                  class="control-item"
                  type="button"
                  @click="go(tool.tag)"
                >
                  <span class="control-item-surface">
                    <span class="control-item-icon">
                      <AppIcon :name="tool.icon" :size="32" />
                    </span>
                    <span class="control-item-text">
                      <span class="control-item-title">{{ i18n.t(tool.titleKey) }}</span>
                      <span class="control-item-subtitle">{{ i18n.t(tool.descKey) }}</span>
                    </span>
                  </span>
                </button>
              </div>
            </section>
          </WinCase>

          <WinCase Value="hardware">
            <section class="sample-panel">
              <div class="grid-view">
                <div v-for="card in hardwareCards" :key="card.label" class="control-item static">
                  <span class="control-item-surface">
                    <span class="control-item-icon">
                      <AppIcon :name="card.icon" :size="32" />
                    </span>
                    <span class="control-item-text">
                      <span class="control-item-title">{{ card.label }}</span>
                      <span class="control-item-subtitle wrap">{{ card.value }}</span>
                    </span>
                  </span>
                </div>
              </div>
            </section>
          </WinCase>
        </WinSwitchPresenter>
      </div>
    </div>
  </WinScrollViewer>
</template>

<style scoped>
.home-page {
  display: grid;
  grid-template-rows: auto auto 1fr;
  width: 100%;
  min-width: 0;
  margin: 0;
  overflow-x: hidden;
}

.home-page-header {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  grid-template-rows: auto auto 1fr;
  min-height: 400px;
  overflow: hidden;
}

.home-header-image-mask {
  position: relative;
  grid-column: 1;
  grid-row: 1 / 4;
  height: 400px;
  align-self: stretch;
  mask-image: linear-gradient(to bottom, #000 0%, #000 75%, transparent 85%, transparent 100%);
  overflow: hidden;
}

.home-header-image-grid {
  position: absolute;
  inset: -100px 0 0 0;
  height: 500px;
  background: linear-gradient(to bottom, #ced8e4 0%, #d5dbe3 100%);
}

.home-header-image {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(1200px 500px at 85% -10%, rgba(255, 255, 255, 0.6), transparent 60%),
    radial-gradient(900px 480px at 8% 115%, rgba(0, 90, 158, 0.28), transparent 55%),
    linear-gradient(180deg, #d9e3ee 0%, #cdd8e4 62%, #c5d2e0 100%);
}

html.theme-dark .home-header-image-grid {
  background: #020b20;
}

html.theme-dark .home-header-image {
  background:
    radial-gradient(1200px 520px at 85% -10%, rgba(96, 150, 255, 0.2), transparent 60%),
    radial-gradient(900px 480px at 8% 115%, rgba(0, 40, 120, 0.4), transparent 55%),
    linear-gradient(180deg, #05122b 0%, #020b20 100%);
}

@media (prefers-color-scheme: dark) {
  html:not(.theme-light) .home-header-image-grid {
    background: #020b20;
  }

  html:not(.theme-light) .home-header-image {
    background:
      radial-gradient(1200px 520px at 85% -10%, rgba(96, 150, 255, 0.2), transparent 60%),
      radial-gradient(900px 480px at 8% 115%, rgba(0, 40, 120, 0.4), transparent 55%),
      linear-gradient(180deg, #05122b 0%, #020b20 100%);
  }
}

.home-header-copy {
  position: relative;
  grid-column: 1;
  grid-row: 1;
  align-self: center;
  z-index: 1;
  margin: 48px 0 0 36px;
  display: flex;
  flex-direction: column;
}

.home-header-subtitle {
  color: var(--text-primary);
}

.home-header-title {
  color: var(--text-primary);
}

.home-header-tiles-scroll {
  position: relative;
  grid-column: 1;
  grid-row: 3;
  align-self: start;
  z-index: 1;
  margin-top: 76px;
  height: 172px;
  min-width: 0;
  max-width: 100%;
  box-sizing: border-box;
}

.home-header-tiles {
  display: flex;
  gap: 12px;
  width: max-content;
}

.win-home-header-tile {
  position: relative;
  width: 232px;
  height: 172px;
  flex: 0 0 232px;
  box-sizing: border-box;
  overflow: hidden;
  padding: 0;
  margin: 0;
  display: block;
  text-align: left;
  color: var(--text-primary);
  background: transparent;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font: inherit;
  isolation: isolate;
  -webkit-backdrop-filter: blur(30px);
  backdrop-filter: blur(30px);
}

.win-home-header-tile::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  border-radius: inherit;
  background: color-mix(in srgb, rgba(252, 252, 252, 1) 80%, transparent);
}

:global(html.theme-dark .win-home-header-tile::before) {
  background: rgba(44, 44, 44, 0.8);
}

@media (prefers-color-scheme: dark) {
  :global(html:not(.theme-light) .win-home-header-tile::before) {
    background: rgba(44, 44, 44, 0.8);
  }
}

.win-home-header-tile:hover:not(:active) {
  color: var(--text-primary);
  background: var(--subtle-fill-color-secondary, rgba(0, 0, 0, 0.06));
  border-color: var(--control-stroke-color-secondary, rgba(0, 0, 0, 0.16));
}

.win-home-header-tile:hover:not(:active)::before {
  background: color-mix(in srgb, rgba(0, 0, 0, 0.04), transparent);
}

html.theme-dark .win-home-header-tile:hover:not(:active)::before {
  background: color-mix(in srgb, rgba(255, 255, 255, 0.06), transparent);
}

.win-home-header-tile:active {
  color: var(--text-secondary);
}

.win-home-header-tile:active::before {
  background: color-mix(in srgb, rgba(0, 0, 0, 0.06), transparent);
}

.win-home-header-tile-content {
  position: relative;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 20px;
  display: grid;
  grid-template-rows: 44px minmax(0, 1fr);
  row-gap: 14px;
  text-align: left;
  border: 1px solid var(--control-stroke-color-default, rgba(0, 0, 0, 0.08));
  border-radius: 7px;
}

html.theme-dark .win-home-header-tile-content {
  border-color: var(--control-stroke-color-default, rgba(255, 255, 255, 0.08));
}

.win-home-header-tile-source {
  width: 40px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--AccentButtonBackground, #005fb8);
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 14%, transparent);
  border-radius: 10px;
}

:global(html.theme-dark .win-home-header-tile-source) {
  color: #4cc2ff;
  background: color-mix(in srgb, #4cc2ff 16%, transparent);
}

.win-home-header-tile-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.win-home-header-tile-title {
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 600;
  line-height: 20px;
}

.win-home-header-tile-description {
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 16px;
  display: -webkit-box;
  max-height: 48px;
  overflow: hidden;
  line-clamp: 3;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
}

.win-home-header-tile-open-icon {
  position: absolute;
  right: 12px;
  bottom: 12px;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 14px;
  pointer-events: none;
}

.filter-bar {
  justify-self: center;
  align-self: center;
  width: max-content;
  max-width: 100%;
  margin: 24px 0 16px 36px;
}

.token-filter-bar {
  --SelectorBarItemSpacing: 8px;
  --SelectorBarItemIconScale: 0.8;
  --ControlContentThemeFontSize: 14px;
  --TokenViewSelectorBarTextFontFamily: "Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", system-ui, sans-serif;
  gap: 8px;
}

.token-filter-bar.is-cjk-locale {
  --TokenViewSelectorBarTextFontFamily: "Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
}

.token-filter-bar :deep(.win-selector-bar-items-view) {
  gap: 8px;
  padding: 4px 0;
}

.token-filter-bar :deep(.win-selector-bar-item) {
  box-sizing: border-box;
  width: auto;
  height: 32px;
  min-height: 32px;
  padding: 0;
  grid-template-rows: auto;
  align-items: center;
  justify-items: center;
  color: var(--text-primary);
  background: var(--control-fill-color-default, var(--ctrl-fill-default));
  border: 1px solid var(--control-stroke-color-default, var(--ctrl-border));
  border-radius: 16px;
  line-height: 20px;
  font-size: 14px;
  font-weight: 400;
  font-family: var(--TokenViewSelectorBarTextFontFamily);
}

.token-filter-bar :deep(.win-selector-bar-item-content) {
  grid-row: 1;
  grid-column: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: max-content;
  height: 20px;
  margin: 5px 23px;
  line-height: 1;
}

.token-filter-bar :deep(.win-selector-bar-item-text) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 20px;
  font-size: 14px;
  font-weight: 400;
  line-height: 20px;
  font-family: var(--TokenViewSelectorBarTextFontFamily);
  transform: none;
  vertical-align: top;
}

.token-filter-bar :deep(.win-selector-bar-item-icon) {
  width: 20px;
  height: 20px;
  margin: 0 -2px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 20px;
  font-size: 20px;
  line-height: 20px;
}

.token-filter-bar :deep(.win-selector-bar-item-icon-glyph) {
  width: 20px;
  height: 20px;
  font-size: inherit;
  line-height: inherit;
}

.token-filter-bar :deep(.win-selector-bar-item:hover) {
  color: var(--text-primary);
  background: var(--control-fill-color-secondary, var(--ctrl-fill-secondary));
}

.token-filter-bar :deep(.win-selector-bar-item:active) {
  color: var(--text-secondary);
  background: var(--control-fill-color-secondary, var(--ctrl-fill-secondary));
}

.token-filter-bar :deep(.win-selector-bar-item.is-selected) {
  color: var(--accent-text);
  background: var(--accent-base);
  border-color: var(--accent-base);
  font-weight: 400;
}

.token-filter-bar :deep(.win-selector-bar-item.is-selected:hover) {
  color: var(--accent-text);
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}

.token-filter-bar :deep(.win-selector-bar-item.is-selected:active) {
  color: var(--accent-text-secondary);
  background: var(--accent-pressed);
  border-color: var(--accent-pressed);
}

.token-filter-bar :deep(.win-selector-bar-item-selection-visual) {
  display: none;
}

.switch-presenter {
  position: relative;
  min-width: 0;
  margin: 0 36px 36px 36px;
}

.sample-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.grid-view {
  display: grid;
  grid-template-columns: repeat(auto-fill, 300px);
  gap: 12px;
  justify-content: start;
  min-width: 0;
}

.control-item {
  width: 300px;
  height: 96px;
  box-sizing: border-box;
  padding: 0;
  display: block;
  text-align: left;
  color: var(--text-primary);
  background: transparent;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font: inherit;
}

.control-item.static {
  cursor: default;
}

.control-item-surface {
  position: relative;
  isolation: isolate;
  width: 100%;
  height: 100%;
  box-sizing: border-box;
  padding: 8px;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  column-gap: 0;
  color: inherit;
  background: transparent;
  border-radius: 8px;
  border: 1px solid var(--card-stroke, rgba(0, 0, 0, 0.08));
}

:global(html.theme-dark .control-item-surface) {
  border-color: var(--card-stroke, rgba(255, 255, 255, 0.08));
}

.control-item-surface::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  border-radius: inherit;
  background: var(--control-item-fill, var(--CardBackgroundFillColorDefaultBrush, var(--card-bg)));
  transition: background 83ms linear;
}

.control-item:hover:not(:active) {
  color: var(--text-primary);
}

.control-item:hover:not(:active) .control-item-surface {
  --control-item-fill: var(--control-fill-color-secondary, var(--ctrl-fill-secondary));
}

.control-item:active {
  color: var(--text-secondary);
}

.control-item:active .control-item-surface {
  --control-item-fill: var(--control-fill-color-tertiary, var(--ctrl-fill-tertiary));
}

.control-item-icon {
  position: relative;
  width: 40px;
  height: 40px;
  margin: 12px 12px 0 8px;
  align-self: start;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--AccentButtonBackground, #005fb8);
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 14%, transparent);
  border-radius: 10px;
}

:global(html.theme-dark .control-item-icon) {
  color: #4cc2ff;
  background: color-mix(in srgb, #4cc2ff 16%, transparent);
}

.control-item-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.control-item-title {
  margin-top: 14px;
  color: inherit;
  font-size: 14px;
  font-weight: 600;
  line-height: 20px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.control-item-subtitle {
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 16px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.control-item-subtitle.wrap {
  margin-top: 4px;
  white-space: normal;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

@media (max-width: 640px) {
  .grid-view {
    grid-template-columns: minmax(0, 1fr);
  }

  .control-item {
    width: auto;
    height: auto;
  }

  .home-header-copy {
    margin: 32px 0 0 24px;
  }

  .home-header-tiles-scroll {
    margin-top: 64px;
  }

  .filter-bar {
    margin: 24px 0 16px 24px;
  }

  .switch-presenter {
    margin: 0 24px 24px 24px;
  }
}
</style>
