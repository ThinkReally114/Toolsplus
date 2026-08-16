<script setup lang="ts">
import { inject, computed } from "vue";
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
  setTheme("system");
  location.reload();
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
</style>
