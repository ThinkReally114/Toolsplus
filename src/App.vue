<script setup lang="ts">
import { computed, provide, h, ref, watch, onMounted } from "vue";
import { useRouter } from "vue-router";
import WinNavigationView from "@winui/components/WinNavigationView.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinContentDialog from "@winui/components/WinContentDialog.vue";
import { invoke } from "@tauri-apps/api/core";
import { createI18n, i18nKey } from "@winui/components/i18n/index";
import {
  NavigationTrigger_NavigatingTo,
  NavigationTrigger_NavigatingAway,
  NavigationTrigger_BackNavigatingTo,
  NavigationTrigger_BackNavigatingAway,
  DefaultNavigationTransitionInfo,
  createEntranceNavigationTransitionInfo,
  createDrillInNavigationTransitionInfo,
  createSuppressNavigationTransitionInfo,
  createSlideNavigationTransitionInfo,
  createCommonNavigationTransitionInfo,
  createContinuumNavigationTransitionInfo,
  getNavigationTransitionInfoClassName,
  normalizeNavigationTransitionInfo,
  parseNavigationTransitionInfo,
  stringifyNavigationTransitionInfo,
  navigationTransitionInfoEquals,
} from "@winui/utils/navigationTransitionInfo.js";
import { toolboxZhCN, toolboxEnUS } from "./i18n/resources";
import { useTheme } from "./composables/useTheme";
import AppIcon from "@/components/AppIcon.vue";
import TitleBar from "@/components/TitleBar.vue";

const { mode, setTheme } = useTheme();

const savedLang = localStorage.getItem("toolsplus-lang");
const i18n = createI18n(savedLang || navigator.language, {
  "zh-CN": toolboxZhCN as Record<string, string>,
  "en-US": toolboxEnUS as Record<string, string>,
});
provide(i18nKey, i18n);
const { t } = i18n;

const router = useRouter();

const icon = (name: string) => h(AppIcon, { name, size: 16 });

const menuItems = computed(() => [
  { Tag: "home", Icon: icon("home"), Content: t("nav.home") },
  {
    Icon: icon("folder"),
    Content: t("nav.groupUtilities"),
    MenuItems: [
      { Tag: "diskclean", Icon: icon("diskclean"), Content: t("nav.diskclean") },
      { Tag: "performance", Icon: icon("performance"), Content: t("nav.performance") },
      { Tag: "process", Icon: icon("process"), Content: t("nav.process") },
      { Tag: "optimize", Icon: icon("optimize"), Content: t("nav.optimize") },
    ],
  },
  {
    Icon: icon("folderDev"),
    Content: t("nav.groupDev"),
    MenuItems: [
      { Tag: "git", Icon: icon("git"), Content: t("nav.git") },
    ],
  },
]);

const footerMenuItems = computed(() => [
  { Tag: "settings", Icon: icon("settings"), Content: t("nav.settings") },
]);

const selectedItem = computed(() => {
  const name = router.currentRoute.value.name;
  return name ? String(name) : "home";
});

// 侧边栏开关：受控的 IsPaneOpen，切换标签页后自动收起
const isPaneOpen = ref(true);
const navRef = ref<any>(null);

// 导航栏位置：Left / Top / Right
const NAV_MODE_KEY = "toolsplus-nav-mode";
const navMode = ref<"Left" | "Top" | "Right">(
  (localStorage.getItem(NAV_MODE_KEY) as any) || "Left"
);
const paneDisplayMode = computed(() =>
  navMode.value === "Top" ? "Top" : navMode.value === "Right" ? "Left" : "Left"
);
const isRightPane = computed(() => navMode.value === "Right");
function setNavMode(mode: "Left" | "Top" | "Right") {
  navMode.value = mode;
  localStorage.setItem(NAV_MODE_KEY, mode);
}

// 页面过渡动画设置（照搬 WinUIonWeb 官方 Gallery 方案）
const TRANSITION_KEY = "toolsplus-page-transition";
const navigationTransitionInfo = ref(
  parseNavigationTransitionInfo(
    localStorage.getItem(TRANSITION_KEY),
    createEntranceNavigationTransitionInfo()
  )
);
provide("navigationTransitionInfo", navigationTransitionInfo);

const TRANSITION_DURATIONS: Record<string, { enter: number; leave: number }> = {
  DefaultNavigationTransitionInfo: { enter: 450, leave: 150 },
  EntranceNavigationTransitionInfo: { enter: 450, leave: 150 },
  DrillInNavigationTransitionInfo: { enter: 800, leave: 120 },
  SuppressNavigationTransitionInfo: { enter: 0, leave: 0 },
  SlideNavigationTransitionInfo: { enter: 450, leave: 150 },
  CommonNavigationTransitionInfo: { enter: 560, leave: 140 },
  ContinuumNavigationTransitionInfo: { enter: 620, leave: 260 },
};

const transitionDuration = computed(() => {
  const normalized = normalizeNavigationTransitionInfo(
    navigationTransitionInfo.value
  );
  const key = normalized ? normalized.Type : "DefaultNavigationTransitionInfo";
  return TRANSITION_DURATIONS[key] ?? TRANSITION_DURATIONS.DefaultNavigationTransitionInfo;
});

const pageTransitionEnter = ref("");
const pageTransitionLeave = ref("");

watch(
  navigationTransitionInfo,
  (v) => {
    localStorage.setItem(TRANSITION_KEY, stringifyNavigationTransitionInfo(v));
    pageTransitionEnter.value = getNavigationTransitionInfoClassName(
      v,
      NavigationTrigger_NavigatingTo
    );
    pageTransitionLeave.value = getNavigationTransitionInfoClassName(
      v,
      NavigationTrigger_NavigatingAway
    );
  },
  { immediate: true }
);

router.afterEach((to, from) => {
  const historyState = router.options.history.state;
  const isBack = historyState?.forward === from.fullPath;
  pageTransitionEnter.value = getNavigationTransitionInfoClassName(
    navigationTransitionInfo.value,
    isBack ? NavigationTrigger_BackNavigatingTo : NavigationTrigger_NavigatingTo
  );
  pageTransitionLeave.value = getNavigationTransitionInfoClassName(
    navigationTransitionInfo.value,
    isBack ? NavigationTrigger_BackNavigatingAway : NavigationTrigger_NavigatingAway
  );
});

// 动画选项表（供设置页使用）
const transitionOptions = [
  {
    Key: "DefaultNavigationTransitionInfo",
    LabelKey: "settings.transition.default",
    NavigationTransitionInfo: DefaultNavigationTransitionInfo,
  },
  {
    Key: "EntranceNavigationTransitionInfo",
    LabelKey: "settings.transition.entrance",
    NavigationTransitionInfo: createEntranceNavigationTransitionInfo(),
  },
  {
    Key: "DrillInNavigationTransitionInfo",
    LabelKey: "settings.transition.drillIn",
    NavigationTransitionInfo: createDrillInNavigationTransitionInfo(),
  },
  {
    Key: "SuppressNavigationTransitionInfo",
    LabelKey: "settings.transition.suppress",
    NavigationTransitionInfo: createSuppressNavigationTransitionInfo(),
  },
  {
    Key: "SlideNavigationTransitionInfoFromRight",
    LabelKey: "settings.transition.slideRight",
    NavigationTransitionInfo: createSlideNavigationTransitionInfo("FromRight"),
  },
  {
    Key: "SlideNavigationTransitionInfoFromLeft",
    LabelKey: "settings.transition.slideLeft",
    NavigationTransitionInfo: createSlideNavigationTransitionInfo("FromLeft"),
  },
  {
    Key: "CommonNavigationTransitionInfo",
    LabelKey: "settings.transition.common",
    NavigationTransitionInfo: createCommonNavigationTransitionInfo(),
  },
  {
    Key: "ContinuumNavigationTransitionInfo",
    LabelKey: "settings.transition.continuum",
    NavigationTransitionInfo: createContinuumNavigationTransitionInfo(),
  },
];
provide("transitionOptions", transitionOptions);

function onSelectionChanged(e: { SelectedItemContainer?: { Tag?: string } }) {
  const tag = e.SelectedItemContainer?.Tag;
  if (tag && tag !== selectedItem.value) {
    router.push({ name: tag });
  }
}

// 切换标签页后自动收起侧边栏（仅在小窗口 compact 模式下生效）
watch(
  () => router.currentRoute.value.name,
  () => {
    // 通过 ref 调用 WinNavigationView 的内部 ClosePane，仅在 overlay 模式下收起
    const nav = navRef.value;
    if (nav && typeof nav.ClosePane === "function") {
      // 仅当当前处于 compact/overlay 模式才收起，避免大窗口也收起
      // WinNavigationView 暴露的 isClosedCompact 可判断
      if (nav.isClosedCompact) {
        nav.ClosePane();
      }
    }
  }
);

function togglePane() {
  const nav = navRef.value;
  if (nav && typeof nav.toggleCompact === "function") {
    nav.toggleCompact();
  }
}

// 管理员权限检测：启动时若非管理员则弹窗提示提权
const adminDialogOpen = ref(false);
const adminChecking = ref(false);
const ADMIN_KEY = "toolsplus-admin-declined";

async function checkAdminOnStartup() {
  if (typeof (window as any).__TAURI_INTERNALS__ === "undefined") return;
  try {
    const admin = await invoke<boolean>("is_admin");
    if (admin) return;
    if (sessionStorage.getItem(ADMIN_KEY) === "1") return;
    adminDialogOpen.value = true;
  } catch {
    // 命令不可用则跳过
  }
}

async function confirmElevate() {
  adminDialogOpen.value = false;
  adminChecking.value = true;
  try {
    await invoke("relaunch_as_admin");
    exitConfirmed = true;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().destroy();
  } catch (e) {
    console.error("提权失败", e);
    adminChecking.value = false;
  }
}

function declineElevate() {
  adminDialogOpen.value = false;
  sessionStorage.setItem(ADMIN_KEY, "1");
}

checkAdminOnStartup();

// 退出应用：拦截所有关闭请求（标题栏 X / Alt+F4 / 任务栏关闭），弹模态框二次确认
const exitDialogOpen = ref(false);
let exitConfirmed = false;

onMounted(async () => {
  if (typeof (window as any).__TAURI_INTERNALS__ === "undefined") return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const appWindow = getCurrentWindow();
    await appWindow.onCloseRequested((event) => {
      if (exitConfirmed) return;
      event.preventDefault();
      exitDialogOpen.value = true;
    });
  } catch (e) {
    console.error(e);
  }
});

async function confirmExit() {
  exitDialogOpen.value = false;
  exitConfirmed = true;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().destroy();
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <div class="app-root">
    <TitleBar />
    <WinNavigationView
      ref="navRef"
      :PaneDisplayMode="paneDisplayMode"
      :OpenPaneLength="256"
      :MenuItems="menuItems"
      :FooterMenuItems="footerMenuItems"
      :SelectedItem="selectedItem"
      :IsPaneOpen="isPaneOpen"
      @update:IsPaneOpen="(v: boolean) => (isPaneOpen = v)"
      :IsSettingsVisible="false"
      :IsPaneToggleButtonVisible="true"
      :PaneTitle="t('nav.togglePane')"
      IsBackButtonVisible="Collapsed"
      @SelectionChanged="onSelectionChanged"
      class="app-shell"
      :class="{ 'nav-right': isRightPane }"
    >
      <router-view v-slot="{ Component }">
        <transition
          appear
          mode="out-in"
          appear-active-class="EntranceNavigationTransitionInfo"
          :duration="transitionDuration"
          :enter-active-class="pageTransitionEnter"
          :leave-active-class="pageTransitionLeave"
        >
          <component :is="Component" />
        </transition>
      </router-view>
    </WinNavigationView>

    <WinContentDialog
      v-model:IsOpen="adminDialogOpen"
      :Title="t('admin.title')"
      :Content="t('admin.content')"
      :PrimaryButtonText="t('admin.elevate')"
      :CloseButtonText="t('admin.decline')"
      DefaultButton="Primary"
      @PrimaryButtonClick="confirmElevate"
      @CloseButtonClick="declineElevate"
    />

    <WinContentDialog
      v-model:IsOpen="exitDialogOpen"
      :Title="t('exit.title')"
      :Content="t('exit.content')"
      :PrimaryButtonText="t('exit.confirm')"
      :CloseButtonText="t('exit.cancel')"
      DefaultButton="Close"
      @PrimaryButtonClick="confirmExit"
    />
  </div>
</template>

<style>
@font-face {
  font-family: "WinUIOnWebIcons";
  src: url("/SEGOEICONS.TTF")
    format("truetype");
  font-display: block;
}

body .icon,
body .icon-btn,
body .ptr-icon-wrapper,
body .symbol-icon,
body .win-symbol-icon,
body .win-asb-icon,
body .picker-icon,
body .checkbox-glyph,
body .win-combo-chevron,
body .win-cbf-icon,
body .win-cbf-overflow-icon,
body .win-expander-header-icon,
body .win-expander-arrow,
body .infobadge-icon,
body .close-icon,
body .win-menu-flyout-icon,
body .win-menu-flyout-check,
body .win-menu-flyout-check-placeholder,
body .win-menu-flyout-chevron,
body .win-number-spin-button span,
body .win-number-compact-indicator span,
body .win-number-popup-button span,
body .win-password-reveal span,
body .win-rating-glyph,
body .scrollbar-button,
body .win-settings-card-icon,
body .win-settings-card-action-icon,
body .win-teaching-tip-icon,
body .win-teaching-tip-close,
body .win-textbox-delete-glyph,
body .font-icon,
body .icon-glyph,
body .icon-preview-glyph,
body .group-icon,
body .tree-icon {
  font-family: "WinUIOnWebIcons";
}

html,
body,
#app {
  height: 100%;
  margin: 0;
}

#app {
  font-family: "Segoe UI", "Microsoft YaHei UI", system-ui, sans-serif;
}

.app-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.app-shell {
  flex: 1;
  min-height: 0;
}

.app-shell.nav-right {
  flex-direction: row-reverse;
}
</style>