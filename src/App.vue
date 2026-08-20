  1→<script setup lang="ts">
  2→import { computed, provide, h, ref, watch, onMounted } from "vue";
  3→import { useRouter } from "vue-router";
  4→import WinNavigationView from "@winui/components/WinNavigationView.vue";
  5→import WinTextBlock from "@winui/components/WinTextBlock.vue";
  6→import WinContentDialog from "@winui/components/WinContentDialog.vue";
  7→import { invoke } from "@tauri-apps/api/core";
  8→import { createI18n, i18nKey } from "@winui/components/i18n/index";
  9→import {
 10→  NavigationTrigger_NavigatingTo,
 11→  NavigationTrigger_NavigatingAway,
 12→  NavigationTrigger_BackNavigatingTo,
 13→  NavigationTrigger_BackNavigatingAway,
 14→  DefaultNavigationTransitionInfo,
 15→  createEntranceNavigationTransitionInfo,
 16→  createDrillInNavigationTransitionInfo,
 17→  createSuppressNavigationTransitionInfo,
 18→  createSlideNavigationTransitionInfo,
 19→  createCommonNavigationTransitionInfo,
 20→  createContinuumNavigationTransitionInfo,
 21→  getNavigationTransitionInfoClassName,
 22→  normalizeNavigationTransitionInfo,
 23→  parseNavigationTransitionInfo,
 24→  stringifyNavigationTransitionInfo,
 25→  navigationTransitionInfoEquals,
 26→} from "@winui/utils/navigationTransitionInfo.js";
 27→import { toolboxZhCN, toolboxEnUS } from "./i18n/resources";
 28→import { useTheme } from "./composables/useTheme";
 29→import { initBackdrop } from "./composables/useBackdrop";
 30→import { initFont } from "./composables/useFont";
 31→import { initZoom } from "./composables/useZoom";
 32→import AppIcon from "@/components/AppIcon.vue";
 33→import TitleBar from "@/components/TitleBar.vue";
 34→
 35→const { mode, setTheme } = useTheme();
 36→
 37→const savedLang = localStorage.getItem("toolsplus-lang");
 38→const i18n = createI18n(savedLang || navigator.language, {
 39→  "zh-CN": toolboxZhCN as Record<string, string>,
 40→  "en-US": toolboxEnUS as Record<string, string>,
 41→});
 42→provide(i18nKey, i18n);
 43→const { t } = i18n;
 44→
 45→const router = useRouter();
 46→
 47→const icon = (name: string) => h(AppIcon, { name, size: 16 });
 48→
 49→const menuItems = computed(() => [
 50→  { Tag: "home", Icon: icon("home"), Content: t("nav.home") },
 51→  {
 52→    Icon: icon("folder"),
 53→    Content: t("nav.groupUtilities"),
 54→    MenuItems: [
 55→      { Tag: "diskclean", Icon: icon("diskclean"), Content: t("nav.diskclean") },
 56→      { Tag: "performance", Icon: icon("performance"), Content: t("nav.performance") },
 57→      { Tag: "process", Icon: icon("process"), Content: t("nav.process") },
 58→      { Tag: "optimize", Icon: icon("optimize"), Content: t("nav.optimize") },
 59→      { Tag: "visual", Icon: icon("hud"), Content: t("nav.visual") },
 60→    ],
 61→  },
 62→  {
 63→    Icon: icon("folderDev"),
 64→    Content: t("nav.groupDev"),
 65→    MenuItems: [
 66→      { Tag: "git", Icon: icon("git"), Content: t("nav.git") },
 67→    ],
 68→  },
 69→]);
 70→
 71→const footerMenuItems = computed(() => [
 72→  { Tag: "settings", Icon: icon("settings"), Content: t("nav.settings") },
 73→]);
 74→
 75→const selectedItem = computed(() => {
 76→  const name = router.currentRoute.value.name;
 77→  return name ? String(name) : "home";
 78→});
 79→
 80→// 侧边栏开关：受控的 IsPaneOpen，切换标签页后自动收起
 81→const isPaneOpen = ref(true);
 82→const navRef = ref<any>(null);
 83→
 84→// 导航栏位置：Left / Top / Right
 85→const NAV_MODE_KEY = "toolsplus-nav-mode";
 86→const navMode = ref<"Left" | "Top" | "Right">(
 87→  (localStorage.getItem(NAV_MODE_KEY) as any) || "Left"
 88→);
 89→const paneDisplayMode = computed(() =>
 90→  navMode.value === "Top" ? "Top" : navMode.value === "Right" ? "Left" : "Left"
 91→);
 92→const isRightPane = computed(() => navMode.value === "Right");
 93→function setNavMode(mode: "Left" | "Top" | "Right") {
 94→  navMode.value = mode;
 95→  localStorage.setItem(NAV_MODE_KEY, mode);
 96→}
 97→
 98→// 页面过渡动画设置（照搬 WinUIonWeb 官方 Gallery 方案）
 99→const TRANSITION_KEY = "toolsplus-page-transition";
100→const navigationTransitionInfo = ref(
101→  parseNavigationTransitionInfo(
102→    localStorage.getItem(TRANSITION_KEY),
103→    createEntranceNavigationTransitionInfo()
104→  )
105→);
106→provide("navigationTransitionInfo", navigationTransitionInfo);
107→
108→const TRANSITION_DURATIONS: Record<string, { enter: number; leave: number }> = {
109→  DefaultNavigationTransitionInfo: { enter: 280, leave: 120 },
110→  EntranceNavigationTransitionInfo: { enter: 280, leave: 120 },
111→  DrillInNavigationTransitionInfo: { enter: 350, leave: 100 },
112→  SuppressNavigationTransitionInfo: { enter: 0, leave: 0 },
113→  SlideNavigationTransitionInfo: { enter: 280, leave: 120 },
114→  CommonNavigationTransitionInfo: { enter: 320, leave: 110 },
115→  ContinuumNavigationTransitionInfo: { enter: 380, leave: 200 },
116→};
117→
118→const transitionDuration = computed(() => {
119→  const normalized = normalizeNavigationTransitionInfo(
120→    navigationTransitionInfo.value
121→  );
122→  const key = normalized ? normalized.Type : "DefaultNavigationTransitionInfo";
123→  return TRANSITION_DURATIONS[key] ?? TRANSITION_DURATIONS.DefaultNavigationTransitionInfo;
124→});
125→
126→const pageTransitionEnter = ref("");
127→const pageTransitionLeave = ref("");
128→
129→watch(
130→  navigationTransitionInfo,
131→  (v) => {
132→    localStorage.setItem(TRANSITION_KEY, stringifyNavigationTransitionInfo(v));
133→    pageTransitionEnter.value = getNavigationTransitionInfoClassName(
134→      v,
135→      NavigationTrigger_NavigatingTo
136→    );
137→    pageTransitionLeave.value = getNavigationTransitionInfoClassName(
138→      v,
139→      NavigationTrigger_NavigatingAway
140→    );
141→  },
142→  { immediate: true }
143→);
144→
145→router.afterEach((to, from) => {
146→  const historyState = router.options.history.state;
147→  const isBack = historyState?.forward === from.fullPath;
148→  pageTransitionEnter.value = getNavigationTransitionInfoClassName(
149→    navigationTransitionInfo.value,
150→    isBack ? NavigationTrigger_BackNavigatingTo : NavigationTrigger_NavigatingTo
151→  );
152→  pageTransitionLeave.value = getNavigationTransitionInfoClassName(
153→    navigationTransitionInfo.value,
154→    isBack ? NavigationTrigger_BackNavigatingAway : NavigationTrigger_NavigatingAway
155→  );
156→});
157→
158→// 动画选项表（供设置页使用）
159→const transitionOptions = [
160→  {
161→    Key: "DefaultNavigationTransitionInfo",
162→    LabelKey: "settings.transition.default",
163→    NavigationTransitionInfo: DefaultNavigationTransitionInfo,
164→  },
165→  {
166→    Key: "EntranceNavigationTransitionInfo",
167→    LabelKey: "settings.transition.entrance",
168→    NavigationTransitionInfo: createEntranceNavigationTransitionInfo(),
169→  },
170→  {
171→    Key: "DrillInNavigationTransitionInfo",
172→    LabelKey: "settings.transition.drillIn",
173→    NavigationTransitionInfo: createDrillInNavigationTransitionInfo(),
174→  },
175→  {
176→    Key: "SuppressNavigationTransitionInfo",
177→    LabelKey: "settings.transition.suppress",
178→    NavigationTransitionInfo: createSuppressNavigationTransitionInfo(),
179→  },
180→  {
181→    Key: "SlideNavigationTransitionInfoFromRight",
182→    LabelKey: "settings.transition.slideRight",
183→    NavigationTransitionInfo: createSlideNavigationTransitionInfo("FromRight"),
184→  },
185→  {
186→    Key: "SlideNavigationTransitionInfoFromLeft",
187→    LabelKey: "settings.transition.slideLeft",
188→    NavigationTransitionInfo: createSlideNavigationTransitionInfo("FromLeft"),
189→  },
190→  {
191→    Key: "CommonNavigationTransitionInfo",
192→    LabelKey: "settings.transition.common",
193→    NavigationTransitionInfo: createCommonNavigationTransitionInfo(),
194→  },
195→  {
196→    Key: "ContinuumNavigationTransitionInfo",
197→    LabelKey: "settings.transition.continuum",
198→    NavigationTransitionInfo: createContinuumNavigationTransitionInfo(),
199→  },
200→];
201→provide("transitionOptions", transitionOptions);
202→
203→function onSelectionChanged(e: { SelectedItemContainer?: { Tag?: string } }) {
204→  const tag = e.SelectedItemContainer?.Tag;
205→  if (tag && tag !== selectedItem.value) {
206→    router.push({ name: tag });
207→  }
208→}
209→
210→// 切换标签页后自动收起侧边栏（仅在小窗口 compact 模式下生效）
211→watch(
212→  () => router.currentRoute.value.name,
213→  () => {
214→    // 通过 ref 调用 WinNavigationView 的内部 ClosePane，仅在 overlay 模式下收起
215→    const nav = navRef.value;
216→    if (nav && typeof nav.ClosePane === "function") {
217→      // 仅当当前处于 compact/overlay 模式才收起，避免大窗口也收起
218→      // WinNavigationView 暴露的 isClosedCompact 可判断
219→      if (nav.isClosedCompact) {
220→        nav.ClosePane();
221→      }
222→    }
223→  }
224→);
225→
226→function togglePane() {
227→  const nav = navRef.value;
228→  if (nav && typeof nav.toggleCompact === "function") {
229→    nav.toggleCompact();
230→  }
231→}
232→
233→// 管理员权限检测：启动时若非管理员则弹窗提示提权
234→const adminDialogOpen = ref(false);
235→const adminChecking = ref(false);
236→const ADMIN_KEY = "toolsplus-admin-declined";
237→
238→async function checkAdminOnStartup() {
239→  if (typeof (window as any).__TAURI_INTERNALS__ === "undefined") return;
240→  try {
241→    const admin = await invoke<boolean>("is_admin");
242→    if (admin) return;
243→    if (sessionStorage.getItem(ADMIN_KEY) === "1") return;
244→    adminDialogOpen.value = true;
245→  } catch {
246→    // 命令不可用则跳过
247→  }
248→}
249→
250→async function confirmElevate() {
251→  adminDialogOpen.value = false;
252→  adminChecking.value = true;
253→  try {
254→    await invoke("relaunch_as_admin");
255→    exitConfirmed = true;
256→    const { getCurrentWindow } = await import("@tauri-apps/api/window");
257→    await getCurrentWindow().destroy();
258→  } catch (e) {
259→    console.error("提权失败", e);
260→    adminChecking.value = false;
261→  }
262→}
263→
264→function declineElevate() {
265→  adminDialogOpen.value = false;
266→  sessionStorage.setItem(ADMIN_KEY, "1");
267→}
268→
269→checkAdminOnStartup();
270→
271→// 退出应用：拦截所有关闭请求（标题栏 X / Alt+F4 / 任务栏关闭），弹模态框二次确认
272→const exitDialogOpen = ref(false);
273→let exitConfirmed = false;
274→
275→onMounted(async () => {
276→  requestAnimationFrame(() => {
277→    initBackdrop();
278→    initFont();
279→    initZoom();
280→  });
281→  import("@/views/HomeView.vue").catch(() => {});
282→  if (typeof (window as any).__TAURI_INTERNALS__ === "undefined") return;
283→  try {
284→    const { getCurrentWindow } = await import("@tauri-apps/api/window");
285→    const appWindow = getCurrentWindow();
286→    await appWindow.onCloseRequested((event) => {
287→      if (exitConfirmed) return;
288→      event.preventDefault();
289→      exitDialogOpen.value = true;
290→    });
291→    await setupKeystrokesGlobal();
292→  } catch (e) {
293→    console.error(e);
294→  }
295→});
296→
297→async function setupKeystrokesGlobal() {
298→  const { invoke } = await import("@tauri-apps/api/core");
299→  const ksEnabled = () => localStorage.getItem("ks.enabled") !== "0";
300→  const enabled = ksEnabled();
301→  try {
302→    await invoke("set_ks_enabled", { enabled });
303→    if (enabled) {
304→      const { Window } = await import("@tauri-apps/api/window");
305→      const glass = await Window.getByLabel("glass");
306→      if (glass && !(await glass.isVisible())) {
307→        await glass.show();
308→      }
309→    }
310→  } catch (e) {
311→    console.error("全局钩子初始化失败", e);
312→  }
313→}
314→
315→async function confirmExit() {
316→  exitDialogOpen.value = false;
317→  exitConfirmed = true;
318→  try {
319→    const { getCurrentWindow } = await import("@tauri-apps/api/window");
320→    await getCurrentWindow().destroy();
321→  } catch (e) {
322→    console.error(e);
323→  }
324→}
325→</script>
326→
327→<template>
328→  <div class="app-root">
329→    <TitleBar />
330→    <WinNavigationView
331→      ref="navRef"
332→      :PaneDisplayMode="paneDisplayMode"
333→      :OpenPaneLength="256"
334→      :MenuItems="menuItems"
335→      :FooterMenuItems="footerMenuItems"
336→      :SelectedItem="selectedItem"
337→      :IsPaneOpen="isPaneOpen"
338→      @update:IsPaneOpen="(v: boolean) => (isPaneOpen = v)"
339→      :IsSettingsVisible="false"
340→      :IsPaneToggleButtonVisible="true"
341→      :PaneTitle="t('nav.togglePane')"
342→      IsBackButtonVisible="Collapsed"
343→      @SelectionChanged="onSelectionChanged"
344→      class="app-shell"
345→      :class="{ 'nav-right': isRightPane }"
346→    >
347→      <router-view v-slot="{ Component }">
348→        <transition
349→          appear
350→          mode="out-in"
351→          appear-active-class="EntranceNavigationTransitionInfo"
352→          :duration="transitionDuration"
353→          :enter-active-class="pageTransitionEnter"
354→          :leave-active-class="pageTransitionLeave"
355→        >
356→          <component :is="Component" />
357→        </transition>
358→      </router-view>
359→    </WinNavigationView>
360→
361→    <WinContentDialog
362→      v-model:IsOpen="adminDialogOpen"
363→      :Title="t('admin.title')"
364→      :Content="t('admin.content')"
365→      :PrimaryButtonText="t('admin.elevate')"
366→      :CloseButtonText="t('admin.decline')"
367→      DefaultButton="Primary"
368→      @PrimaryButtonClick="confirmElevate"
369→      @CloseButtonClick="declineElevate"
370→    />
371→
372→    <WinContentDialog
373→      v-model:IsOpen="exitDialogOpen"
374→      :Title="t('exit.title')"
375→      :Content="t('exit.content')"
376→      :PrimaryButtonText="t('exit.confirm')"
377→      :CloseButtonText="t('exit.cancel')"
378→      DefaultButton="Close"
379→      @PrimaryButtonClick="confirmExit"
380→    />
381→  </div>
382→</template>
383→
384→<style>
385→@font-face {
386→  font-family: "WinUIOnWebIcons";
387→  src: url("/SEGOEICONS.TTF")
388→    format("truetype");
389→  font-display: block;
390→}
391→
392→body .icon,
393→body .icon-btn,
394→body .ptr-icon-wrapper,
395→body .symbol-icon,
396→body .win-symbol-icon,
397→body .win-asb-icon,
398→body .picker-icon,
399→body .checkbox-glyph,
400→body .win-combo-chevron,
401→body .win-cbf-icon,
402→body .win-cbf-overflow-icon,
403→body .win-expander-header-icon,
404→body .win-expander-arrow,
405→body .infobadge-icon,
406→body .close-icon,
407→body .win-menu-flyout-icon,
408→body .win-menu-flyout-check,
409→body .win-menu-flyout-check-placeholder,
410→body .win-menu-flyout-chevron,
411→body .win-number-spin-button span,
412→body .win-number-compact-indicator span,
413→body .win-number-popup-button span,
414→body .win-password-reveal span,
415→body .win-rating-glyph,
416→body .scrollbar-button,
417→body .win-settings-card-icon,
418→body .win-settings-card-action-icon,
419→body .win-teaching-tip-icon,
420→body .win-teaching-tip-close,
421→body .win-textbox-delete-glyph,
422→body .font-icon,
423→body .icon-glyph,
424→body .icon-preview-glyph,
425→body .group-icon,
426→body .tree-icon {
427→  font-family: "WinUIOnWebIcons";
428→}
429→
430→html,
431→body {
432→  height: 100dvh !important;
433→  margin: 0;
434→  font-family: var(--app-font-family, "Segoe UI", "Microsoft YaHei UI", system-ui, sans-serif);
435→  -webkit-font-smoothing: var(--app-font-smoothing, auto);
436→  -moz-osx-font-smoothing: grayscale;
437→  text-rendering: optimizeLegibility;
438→  transition: background-color 0.4s cubic-bezier(0.16, 1, 0.3, 1);
439→}
440→
441→#app {
442→  height: 100dvh !important;
443→  margin: 0;
444→  transition: background-color 0.4s cubic-bezier(0.16, 1, 0.3, 1);
445→}
446→
447→.app-root {
448→  display: flex;
449→  flex-direction: column;
450→  height: 100dvh;
451→  overflow: hidden;
452→}
453→
454→.app-shell {
455→  flex: 1;
456→  min-height: 0;
457→}
458→
459→.app-shell.nav-right {
460→  flex-direction: row-reverse;
461→}
462→</style>