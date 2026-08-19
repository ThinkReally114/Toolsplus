<script setup lang="ts">
import { inject, computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
import WinExpander from "@winui/components/WinExpander.vue";
import WinRadioButton from "@winui/components/WinRadioButton.vue";
import WinRadioButtons from "@winui/components/WinRadioButtons.vue";
import WinSettingsCard from "@winui/components/WinSettingsCard.vue";
import WinComboBox from "@winui/components/WinComboBox.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinButton from "@winui/components/WinButton.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";
import { navigationTransitionInfoEquals } from "@winui/utils/navigationTransitionInfo.js";
import { useTheme, type ThemeMode } from "@/composables/useTheme";
import {
  applyBackdrop,
  applyBodyOpacity,
  getBackdrop,
  getOpacity,
  getKeepBlur,
  setKeepBlur,
  type BackdropType,
} from "@/composables/useBackdrop";
import {
  fontFamily as fontValue,
  antialias as aaValue,
  setFontFamily,
  setAntialias,
  type FontFamily,
  type AntialiasMode,
} from "@/composables/useFont";
import { webviewZoom, applyZoom } from "@/composables/useZoom";
import {
  getHomeBackgroundPath,
  clearHomeBackground,
  setHomeBackground,
  homeBackgroundOpacity,
  setHomeBackgroundOpacity,
} from "@/composables/useHomeBackground";
import AppIcon from "@/components/AppIcon.vue";

const i18n = inject<I18n>(i18nKey)!;
const { mode, setTheme } = useTheme();

const navigationTransitionInfo = inject<any>("navigationTransitionInfo")!;
const transitionOptions = inject<any[]>("transitionOptions")!;

const transitionIndex = computed(() => {
  const idx = transitionOptions.findIndex((o) =>
    navigationTransitionInfoEquals(
      navigationTransitionInfo.value,
      o.NavigationTransitionInfo
    )
  );
  return idx >= 0 ? idx : 0;
});

function onTransitionChange(e: { SelectedIndex?: number }) {
  const opt = transitionOptions[e.SelectedIndex ?? 0];
  if (opt) navigationTransitionInfo.value = opt.NavigationTransitionInfo;
}

const themeOptions = computed(() => [
  { Tag: "system", Content: i18n.t("settings.theme.system") },
  { Tag: "light", Content: i18n.t("settings.theme.light") },
  { Tag: "dark", Content: i18n.t("settings.theme.dark") },
]);

const themeIndex = computed(() => {
  const idx = themeOptions.value.findIndex((o) => o.Tag === mode.value);
  return idx >= 0 ? idx : 0;
});

function onThemeChange(e: { SelectedIndex?: number }) {
  const idx = e.SelectedIndex ?? 0;
  const tag = themeOptions.value[idx]?.Tag as ThemeMode | undefined;
  if (tag) setTheme(tag);
}

const langOptions = computed(() => [
  { label: "简体中文", value: "zh-CN" },
  { label: "English", value: "en-US" },
]);
const langValue = computed(() =>
  i18n.locale.startsWith("zh") ? "zh-CN" : "en-US"
);
function onLangChange(e: { AddedItems?: any[] }) {
  const item = e?.AddedItems?.[0];
  const tag = item?.value;
  if (tag) {
    localStorage.setItem("toolsplus-lang", tag);
    location.reload();
  }
}

const NAV_POS_KEY = "toolsplus-nav-mode";
const navPosList = ["Left", "Top", "Right"] as const;
const navPosItems = navPosList.map((v) => ({ Text: v, Value: v }));
const navPosIndex = computed(() => {
  const v = (localStorage.getItem(NAV_POS_KEY) as any) || "Left";
  const idx = navPosList.indexOf(v);
  return idx >= 0 ? idx : 0;
});
function onNavPosChange(e: { SelectedIndex?: number; AddedItems?: any[] }) {
  const val =
    (e.AddedItems && e.AddedItems[0]) ||
    (e.SelectedIndex != null && navPosList[e.SelectedIndex]) ||
    navPosList[0];
  localStorage.setItem(NAV_POS_KEY, val);
  location.reload();
}

function reset() {
  localStorage.removeItem("toolsplus-theme");
  localStorage.removeItem("toolsplus-lang");
  localStorage.removeItem("toolsplus-backdrop");
  localStorage.removeItem("toolsplus-opacity");
  localStorage.removeItem("toolsplus-font");
  localStorage.removeItem("toolsplus-antialias");
  localStorage.removeItem("toolsplus-webview-zoom");
  localStorage.removeItem("toolsplus-home-bg");
  localStorage.removeItem("toolsplus-home-bg-opacity");
  setTheme("system");
  location.reload();
}

const backdropOptions = computed(() => [
  { label: i18n.t("settings.backdrop.none"), value: "none" },
  { label: i18n.t("settings.backdrop.mica"), value: "mica" },
]);

const backdropValue = ref(getBackdrop());
const opacityValue = ref(getOpacity());
const keepBlur = ref(getKeepBlur());
function onKeepBlurChange(v: boolean) {
  keepBlur.value = v;
  setKeepBlur(v);
  if (!v) {
    applyBodyOpacity(getOpacity());
  }
}

function onBackdropChange(e: { AddedItems?: any[] }) {
  const item = e?.AddedItems?.[0];
  const tag = item?.value as BackdropType | undefined;
  if (tag) {
    backdropValue.value = tag;
    applyBackdrop(tag, opacityValue.value);
  }
}

function onOpacityChange(e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  opacityValue.value = v;
  applyBackdrop(backdropValue.value, v);
}

const fontOptions = computed(() => [
  { label: i18n.t("settings.font.system"), value: "system" },
  { label: "Segoe UI", value: "segoe" },
  { label: i18n.t("settings.font.yahei"), value: "yahei" },
  { label: i18n.t("settings.font.simsun"), value: "simsun" },
  { label: i18n.t("settings.font.kaiti"), value: "kaiti" },
  { label: i18n.t("settings.font.consolas"), value: "consolas" },
]);
const fontSel = computed({
  get: () => fontValue.value,
  set: (v: FontFamily) => setFontFamily(v),
});

const aaOptions = computed(() => [
  { label: i18n.t("settings.aa.auto"), value: "auto" },
  { label: i18n.t("settings.aa.antialiased"), value: "antialiased" },
  { label: i18n.t("settings.aa.subpixel"), value: "subpixel-antialiased" },
  { label: i18n.t("settings.aa.none"), value: "none" },
]);
const aaSel = computed({
  get: () => aaValue.value,
  set: (v: AntialiasMode) => setAntialias(v),
});

const zoomValue = ref(webviewZoom.value);
function onZoomChange(e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  zoomValue.value = v;
  applyZoom(v);
}

const bgPath = ref(getHomeBackgroundPath() || "");
const bgOpacityValue = ref(homeBackgroundOpacity().value);
const bgUrl = ref<string | null>(null);
const hasTauriForBg =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function loadBgPreview() {
  if (!bgPath.value || !hasTauriForBg) {
    bgUrl.value = null;
    return;
  }
  try {
    bgUrl.value = await invoke<string>("read_image_as_data_url", {
      path: bgPath.value,
    });
  } catch (e) {
    bgUrl.value = null;
    console.error(e);
  }
}
loadBgPreview();

async function onPickImage() {
  if (!hasTauriForBg) return;
  try {
    const picked = await invoke<string | null>("pick_image");
    if (!picked) return;
    bgPath.value = picked;
    await setHomeBackground(picked);
    await loadBgPreview();
  } catch (e) {
    console.error(e);
  }
}

function onClearImage() {
  clearHomeBackground();
  bgPath.value = "";
  bgUrl.value = null;
}

function onBgOpacityChange(e: Event) {
  const v = Number((e.target as HTMLInputElement).value);
  bgOpacityValue.value = v;
  setHomeBackgroundOpacity(v);
}

function openRepo() {
  window.open("https://github.com/", "_blank", "noopener,noreferrer");
}
</script>

<template>
  <WinScrollViewer class="settings-scroll">
    <div class="settings-page">
      <h1 class="settings-title">{{ i18n.t("nav.settings") }}</h1>

      <div class="settings-content">
        <WinTextBlock
          class="settings-section-title"
          :Text="i18n.t('settings.appearance')"
        />
        <div class="settings-controls">
          <WinExpander
            Height="70"
            :Header="i18n.t('settings.theme.title')"
            :Description="i18n.t('settings.theme.desc')"
            HeaderIcon="&#xE771;"
          >
            <WinRadioButtons
              :SelectedIndex="themeIndex"
              @SelectionChanged="onThemeChange"
            >
              <WinRadioButton
                v-for="opt in themeOptions"
                :key="opt.Tag"
                :Content="opt.Content"
              />
            </WinRadioButtons>
          </WinExpander>

          <WinSettingsCard
            :Header="i18n.t('settings.language.title')"
            :Description="i18n.t('settings.language.desc')"
            :HeaderIcon="'\uE716'"
            :Height="70"
          >
            <WinComboBox
              :SelectedValue="langValue"
              :ItemsSource="langOptions"
              DisplayMemberPath="label"
              SelectedValuePath="value"
              @SelectionChanged="onLangChange"
            />
          </WinSettingsCard>

          <WinExpander
            Height="70"
            :Header="i18n.t('settings.navpos.title')"
            :Description="i18n.t('settings.navpos.desc')"
            HeaderIcon="&#xE7C4;"
          >
            <WinRadioButtons
              :ItemsSource="navPosItems"
              :SelectedIndex="navPosIndex"
              @SelectionChanged="onNavPosChange"
            />
          </WinExpander>

          <WinExpander
            Height="70"
            :Header="i18n.t('settings.transition.title')"
            :Description="i18n.t('settings.transition.desc')"
            HeaderIcon="&#xE8AB;"
          >
            <WinRadioButtons
              :SelectedIndex="transitionIndex"
              @SelectionChanged="onTransitionChange"
            >
              <WinRadioButton
                v-for="opt in transitionOptions"
                :key="opt.Key"
                :Content="i18n.t(opt.LabelKey)"
              />
            </WinRadioButtons>
          </WinExpander>

          <WinExpander
            Height="80"
            :Header="i18n.t('settings.backdrop.title')"
            :Description="i18n.t('settings.backdrop.desc')"
            HeaderIcon="&#xE7B3;"
          >
            <div class="backdrop-controls">
              <WinComboBox
                :SelectedValue="backdropValue"
                :ItemsSource="backdropOptions"
                DisplayMemberPath="label"
                SelectedValuePath="value"
                @SelectionChanged="onBackdropChange"
              />
              <div class="opacity-row">
                <WinTextBlock
                  :Text="i18n.t('settings.backdrop.opacity')"
                  Style="font-size:13px"
                />
                <input
                  type="range"
                  min="20"
                  max="100"
                  :value="opacityValue"
                  class="opacity-slider"
                  @input="onOpacityChange"
                />
                <WinTextBlock
                  :Text="opacityValue + '%'"
                  Style="font-size:12px;min-width:36px;text-align:right"
                  Foreground="secondary"
                />
              </div>
              <div class="opacity-row" style="margin-top:8px">
                <WinTextBlock
                  :Text="i18n.t('settings.backdrop.keepBlur')"
                  Style="font-size:13px"
                />
                <input
                  type="checkbox"
                  :checked="keepBlur"
                  @change="onKeepBlurChange(($event.target as HTMLInputElement).checked)"
                  style="width:16px;height:16px;cursor:pointer"
                />
              </div>
            </div>
          </WinExpander>

          <WinExpander
            :Height="bgPath ? 88 : 70"
            :Header="i18n.t('settings.homebg.title')"
            :Description="i18n.t('settings.homebg.desc')"
            HeaderIcon="&#xE8B9;"
          >
            <div class="backdrop-controls">
              <div class="opacity-row">
                <WinButton
                  class="reset-btn"
                  @Click="onPickImage"
                  :Content="i18n.t('settings.homebg.pick')"
                />
                <WinButton
                  v-if="bgPath"
                  class="reset-btn"
                  @Click="onClearImage"
                  :Content="i18n.t('settings.homebg.clear')"
                />
              </div>
              <div v-if="bgUrl" class="bg-preview-row">
                <img :src="bgUrl" class="bg-preview-img" alt="preview" />
                <WinTextBlock
                  :Text="bgPath"
                  Style="font-size:12px;opacity:.7;flex:1;min-width:0;word-break:break-all"
                  Foreground="secondary"
                />
              </div>
              <div v-else class="opacity-row">
                <WinTextBlock
                  :Text="i18n.t('settings.homebg.empty')"
                  Style="font-size:13px;opacity:.7"
                  Foreground="secondary"
                />
              </div>
              <div v-if="bgPath" class="opacity-row">
                <WinTextBlock
                  :Text="i18n.t('settings.backdrop.opacity')"
                  Style="font-size:13px"
                />
                <input
                  type="range"
                  min="10"
                  max="100"
                  :value="bgOpacityValue"
                  class="opacity-slider"
                  @input="onBgOpacityChange"
                />
                <WinTextBlock
                  :Text="bgOpacityValue + '%'"
                  Style="font-size:12px;min-width:36px;text-align:right"
                  Foreground="secondary"
                />
              </div>
            </div>
          </WinExpander>

          <WinExpander
            Height="80"
            :Header="i18n.t('settings.font.title')"
            :Description="i18n.t('settings.font.desc')"
            HeaderIcon="&#xE8D2;"
          >
            <div class="backdrop-controls">
              <WinComboBox
                v-model:SelectedValue="fontSel"
                :ItemsSource="fontOptions"
                DisplayMemberPath="label"
                SelectedValuePath="value"
              />
              <div class="opacity-row">
                <WinTextBlock
                  :Text="i18n.t('settings.aa.title')"
                  Style="font-size:13px"
                />
                <WinComboBox
                  v-model:SelectedValue="aaSel"
                  :ItemsSource="aaOptions"
                  DisplayMemberPath="label"
                  SelectedValuePath="value"
                />
              </div>
            </div>
          </WinExpander>

          <WinExpander
            Height="80"
            :Header="i18n.t('settings.zoom.title')"
            :Description="i18n.t('settings.zoom.desc')"
            HeaderIcon="&#xE7E9;"
          >
            <div class="backdrop-controls">
              <div class="opacity-row">
                <WinTextBlock
                  :Text="i18n.t('settings.zoom.scale')"
                  Style="font-size:13px"
                />
                <input
                  type="range"
                  min="50"
                  max="200"
                  step="10"
                  :value="zoomValue"
                  class="opacity-slider"
                  @input="onZoomChange"
                />
                <WinTextBlock
                  :Text="zoomValue + '%'"
                  Style="font-size:12px;min-width:40px;text-align:right"
                  Foreground="secondary"
                />
              </div>
            </div>
          </WinExpander>
        </div>

        <WinTextBlock
          class="settings-section-title about-title"
          :Text="i18n.t('settings.about.title')"
        />
        <div class="about-controls">
          <WinExpander
            :Header="i18n.t('app.title')"
            :Description="i18n.t('settings.about.copyright')"
            Height="70"
          >
            <template #HeaderIcon>
              <AppIcon name="app" :size="20" />
            </template>
            <template #HeaderControls>
              <WinTextBlock
                :Text="i18n.t('settings.about.version', { version: '0.1.0' })"
                FontSize="14.4"
                Foreground="var(--TextFillColorSecondaryBrush, var(--text-secondary))"
              />
            </template>
            <div class="about-content">
              <WinTextBlock
                :Text="i18n.t('settings.about.desc')"
                Style="font-size:13px;opacity:.7"
                Foreground="secondary"
              />
              <WinButton
                class="reset-btn"
                @Click="reset"
                :Content="i18n.t('settings.reset')"
              />
            </div>
          </WinExpander>
        </div>
      </div>
    </div>
  </WinScrollViewer>
</template>

<style scoped>
.settings-scroll {
  width: 100%;
  height: 100%;
}

.settings-page {
  width: 100%;
  min-width: 0;
  max-width: 1064px;
  margin: 0 auto;
  padding: 24px 36px 48px;
  box-sizing: border-box;
}

.settings-title {
  font-size: 28px;
  font-weight: 600;
  line-height: 36px;
  margin: 0 0 24px;
  color: var(--TextFillColorPrimaryBrush, var(--text-primary, inherit));
}

.settings-content {
  max-width: 1064px;
}

.settings-section-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}

.about-title {
  margin-top: 32px;
}

.settings-controls {
  display: flex;
  flex-direction: column;
}

.settings-controls :deep(.win-expander),
.settings-controls :deep(.win-settings-card) {
  margin-bottom: 4px;
}

.about-controls {
  display: flex;
  flex-direction: column;
  margin-top: 6px;
}

.about-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.reset-btn {
  align-self: flex-start;
}

.backdrop-controls {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.opacity-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.opacity-slider {
  flex: 1;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: var(--ControlStrongFillColorDefaultBrush, rgba(0, 0, 0, 0.2));
  outline: none;
  cursor: pointer;
}

.bg-preview-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bg-preview-img {
  width: 96px;
  height: 48px;
  object-fit: cover;
  border-radius: 4px;
  flex: 0 0 96px;
}

html.theme-dark .opacity-slider {
  background: var(--ControlStrongFillColorDefaultBrush, rgba(255, 255, 255, 0.3));
}

.opacity-slider::-webkit-slider-thumb {
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--AccentButtonBackground, #005fb8);
  border: 2px solid #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
}

html.theme-dark .opacity-slider::-webkit-slider-thumb {
  background: #4cc2ff;
  border-color: #2b2b2b;
}
</style>
