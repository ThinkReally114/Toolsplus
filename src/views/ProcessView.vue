<script setup lang="ts">
import { inject, ref, computed, onMounted, onBeforeUnmount, nextTick, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { sendNotification } from "@tauri-apps/plugin-notification";
import PageShell from "@/components/PageShell.vue";
import AppIcon from "@/components/AppIcon.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinButton from "@winui/components/WinButton.vue";
import WinTextBox from "@winui/components/WinTextBox.vue";
import WinInfoBar from "@winui/components/WinInfoBar.vue";
import WinContentDialog from "@winui/components/WinContentDialog.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;

interface ProcessInfo {
  pid: number;
  name: string;
  cpu_usage: number;
  memory: number;
  memory_percent: number;
  status: string;
  icon: string | null;
  is_self: boolean;
  is_related: boolean;
  is_system: boolean;
  parent_pid: number | null;
}

interface ProcessIconEntry {
  pid: number;
  icon: string | null;
}

interface WindowInfo {
  hwnd: number;
  pid: number;
  title: string;
  class_name: string;
  exe_path: string;
  exe_name: string;
  is_visible: boolean;
  is_topmost: boolean;
  is_click_through: boolean;
  is_minimized: boolean;
  is_maximized: boolean;
  opacity: number;
  icon: string | null;
}

type TabKey = "process" | "window";

const processes = ref<ProcessInfo[]>([]);
const windows = ref<WindowInfo[]>([]);
const activeTab = ref<TabKey>("process");
const loading = ref(false);
const error = ref("");
const searchText = ref("");
const selectedPid = ref<number | null>(null);
const killDialogOpen = ref(false);
const killTargetName = ref("");
const killTargetPid = ref<number | null>(null);
const killTargetIcon = ref<string | null>(null);
const resultDialogOpen = ref(false);
const resultDialogTitle = ref("");
const resultDialogMessage = ref("");
const resultDialogIcon = ref<string | null>(null);
const resultDialogName = ref("");
const resultDialogPid = ref<number | null>(null);
const resultDialogSeverity = ref<"success" | "error">("success");
let iconToken = 0;
let selfPid: number | null = null;

async function refreshSelfPid() {
  try {
    const list = await invoke<ProcessInfo[]>("list_processes");
    const me = list.find((p) => p.is_self);
    selfPid = me ? me.pid : null;
  } catch {
    selfPid = null;
  }
}

function showResult(opts: {
  success: boolean;
  title: string;
  message: string;
  icon: string | null;
  name: string;
  pid: number | null;
}) {
  resultDialogSeverity.value = opts.success ? "success" : "error";
  resultDialogTitle.value = opts.title;
  resultDialogMessage.value = opts.message;
  resultDialogIcon.value = opts.icon;
  resultDialogName.value = opts.name;
  resultDialogPid.value = opts.pid;
  resultDialogOpen.value = true;
}

const pickMode = ref(false);
const pickedPid = ref<number | null>(null);
let pickTimer: number | null = null;
let autoRefreshTimer: number | null = null;
const pickError = ref("");

const ctxMenu = ref<{ x: number; y: number; pid: number; name: string } | null>(null);
const ctxBusy = ref(false);
const pplDialogOpen = ref(false);
const pplContent = ref("");
const pplPid = ref<number | null>(null);
const pplError = ref("");

const winCtxMenu = ref<{ x: number; y: number; w: WindowInfo } | null>(null);
const winCtxBusy = ref(false);
const opacityDialogOpen = ref(false);
const opacityTarget = ref<WindowInfo | null>(null);
const opacityInput = ref(100);

const ctxMenuEl = ref<HTMLElement | null>(null);
const winCtxMenuEl = ref<HTMLElement | null>(null);
const ctxMenuPos = ref<{ left: number; top: number }>({ left: 0, top: 0 });
const winCtxMenuPos = ref<{ left: number; top: number }>({ left: 0, top: 0 });

function clampMenuPos(
  el: HTMLElement | null,
  x: number,
  y: number
): { left: number; top: number } {
  const w = el?.offsetWidth ?? 180;
  const h = el?.offsetHeight ?? 220;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const margin = 8;
  let left = x;
  let top = y;
  if (left + w + margin > vw) left = Math.max(margin, vw - w - margin);
  if (top + h + margin > vh) top = Math.max(margin, vh - h - margin);
  if (left < margin) left = margin;
  if (top < margin) top = margin;
  return { left, top };
}

watch(ctxMenu, async (v) => {
  if (!v) return;
  ctxMenuPos.value = { left: v.x, top: v.y };
  await nextTick();
  ctxMenuPos.value = clampMenuPos(ctxMenuEl.value, v.x, v.y);
});

watch(winCtxMenu, async (v) => {
  if (!v) return;
  winCtxMenuPos.value = { left: v.x, top: v.y };
  await nextTick();
  winCtxMenuPos.value = clampMenuPos(winCtxMenuEl.value, v.x, v.y);
});

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024))
  );
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    running: i18n.t("process.status.running"),
    sleeping: i18n.t("process.status.sleeping"),
    stopped: i18n.t("process.status.stopped"),
    unknown: i18n.t("process.status.unknown"),
  };
  return map[status] || status;
}

async function refresh(skipLoading = false) {
  if (!hasTauri) {
    error.value = "demo";
    return;
  }
  if (!skipLoading) loading.value = true;
  error.value = "";
  try {
    processes.value = await invoke<ProcessInfo[]>("list_processes");
    loadIcons();
    windows.value = await invoke<WindowInfo[]>("list_windows");
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadIcons() {
  const token = ++iconToken;
  const top = processes.value.slice(0, 64).map((p) => p.pid);
  if (top.length === 0) return;
  try {
    const icons = await invoke<ProcessIconEntry[]>("process_icons", {
      pids: top,
    });
    if (token !== iconToken) return;
    const map = new Map(icons.map((i) => [i.pid, i.icon]));
    processes.value = processes.value.map((p) => {
      const icon = map.get(p.pid);
      return icon ? { ...p, icon } : p;
    });
  } catch {
    return;
  }
}

const filteredProcesses = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  if (!q) return processes.value;
  return processes.value.filter(
    (p) =>
      p.name.toLowerCase().includes(q) || String(p.pid).includes(q)
  );
});

interface ProcessNode {
  proc: ProcessInfo;
  children: ProcessNode[];
  isLast: boolean;
  depth: number;
}

const processTree = computed<ProcessNode[]>(() => {
  const q = searchText.value.trim().toLowerCase();
  if (q) return [];
  const byParent = new Map<number | null, ProcessInfo[]>();
  for (const p of processes.value) {
    const key = p.parent_pid ?? null;
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key)!.push(p);
  }
  const roots: ProcessInfo[] = [];
  const allPids = new Set(processes.value.map((p) => p.pid));
  for (const p of processes.value) {
    const parent = p.parent_pid;
    if (parent === null || !allPids.has(parent)) {
      roots.push(p);
    }
  }
  roots.sort((a, b) => {
    const ai = a.is_self ? 0 : a.is_related ? 1 : a.is_system ? 2 : 3;
    const bi = b.is_self ? 0 : b.is_related ? 1 : b.is_system ? 2 : 3;
    if (ai !== bi) return ai - bi;
    return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
  });
  const build = (
    proc: ProcessInfo,
    depth: number,
    isLast: boolean
  ): ProcessNode => {
    const kids = byParent.get(proc.pid) ?? [];
    kids.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
    const children = kids.map((c, i) => build(c, depth + 1, i === kids.length - 1));
    return { proc, children, isLast, depth };
  };
  return roots.map((r, i) => build(r, 0, i === roots.length - 1));
});

function flattenTree(nodes: ProcessNode[]): ProcessNode[] {
  const out: ProcessNode[] = [];
  const walk = (ns: ProcessNode[]) => {
    for (const n of ns) {
      out.push(n);
      walk(n.children);
    }
  };
  walk(nodes);
  return out;
}

const flatProcessTree = computed(() => flattenTree(processTree.value));

const filteredWindows = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  const list = !q
    ? windows.value
    : windows.value.filter(
        (w) =>
          w.title.toLowerCase().includes(q) ||
          w.exe_name.toLowerCase().includes(q) ||
          String(w.pid).includes(q)
      );
  return [...list].sort((a, b) => {
    const ai = a.icon ? 1 : 0;
    const bi = b.icon ? 1 : 0;
    if (bi !== ai) return bi - ai;
    return a.exe_name.toLowerCase().localeCompare(b.exe_name.toLowerCase());
  });
});

const pickedProcesses = computed(() => {
  if (pickedPid.value === null) return [];
  const target = pickedPid.value;
  const related = new Set<number>([target]);
  const queue: number[] = [target];
  while (queue.length > 0) {
    const parent = queue.shift()!;
    for (const p of processes.value) {
      if (related.has(p.pid)) continue;
      if (p.parent_pid === parent) {
        related.add(p.pid);
        queue.push(p.pid);
      }
    }
  }
  const result: ProcessInfo[] = [];
  for (const p of processes.value) {
    if (related.has(p.pid)) {
      result.push({ ...p, is_self: p.pid === target, is_related: p.pid !== target, is_system: false });
    }
  }
  return result;
});

function selectRow(pid: number) {
  selectedPid.value = selectedPid.value === pid ? null : pid;
}

async function killSelected() {
  if (selectedPid.value === null) return;
  killTargetPid.value = selectedPid.value;
  const proc = processes.value.find((p) => p.pid === killTargetPid.value);
  killTargetName.value = proc?.name || String(killTargetPid.value);
  killTargetIcon.value = proc?.icon ?? null;
  killDialogOpen.value = true;
}

async function confirmKill() {
  killDialogOpen.value = false;
  const pid = killTargetPid.value;
  if (pid === null) return;
  const name = killTargetName.value;
  const icon = killTargetIcon.value;
  try {
    await invoke("kill_process", { pid });
    selectedPid.value = null;
    await refresh();
    showResult({
      success: true,
      title: i18n.t("process.killSuccessTitle"),
      message: i18n.t("process.killSuccess", { pid }),
      icon,
      name,
      pid,
    });
  } catch (e) {
    showResult({
      success: false,
      title: i18n.t("process.killFailedTitle"),
      message: i18n.t("process.killFailed", { message: String(e) }),
      icon,
      name,
      pid,
    });
  }
}

function startPickMode() {
  pickMode.value = true;
  pickError.value = "";
  pickedPid.value = null;
  const selfPid = processes.value.find((p) => p.is_self)?.pid ?? null;
  if (pickTimer !== null) {
    clearInterval(pickTimer);
  }
  pickTimer = window.setInterval(async () => {
    if (!pickMode.value) return;
    try {
      const pid = await invoke<number>("get_foreground_window_pid");
      if (selfPid !== null && pid === selfPid) return;
      stopPickMode();
      pickedPid.value = pid;
      await refresh();
      try {
        const win = getCurrentWindow();
        await win.show();
        await win.setFocus();
      } catch {}
      try {
        const proc = processes.value.find((p) => p.pid === pid);
        const name = proc?.name ?? String(pid);
        sendNotification({
          title: i18n.t("process.pickedSuccessTitle"),
          body: i18n.t("process.pickedSuccessBody", { name, pid }),
        });
      } catch {}
    } catch (e) {
      pickError.value = String(e);
    }
  }, 200);
}

function stopPickMode() {
  pickMode.value = false;
  if (pickTimer !== null) {
    clearInterval(pickTimer);
    pickTimer = null;
  }
}

function clearPicked() {
  pickedPid.value = null;
}

function onContextMenu(e: MouseEvent, p: ProcessInfo) {
  e.preventDefault();
  ctxMenu.value = { x: e.clientX, y: e.clientY, pid: p.pid, name: p.name };
}

function closeCtxMenu() {
  ctxMenu.value = null;
}

async function ctxFreeze() {
  if (!ctxMenu.value) return;
  const pid = ctxMenu.value.pid;
  const proc = processes.value.find((p) => p.pid === pid);
  const name = proc?.name || ctxMenu.value.name;
  const icon = proc?.icon ?? null;
  ctxMenu.value = null;
  ctxBusy.value = true;
  try {
    await invoke("suspend_process", { pid });
    await refresh();
    showResult({
      success: true,
      title: i18n.t("process.freezeSuccessTitle"),
      message: i18n.t("process.freezeSuccess", { pid }),
      icon,
      name,
      pid,
    });
  } catch (e) {
    showResult({
      success: false,
      title: i18n.t("process.freezeFailedTitle"),
      message: i18n.t("process.freezeFailed", { message: String(e) }),
      icon,
      name,
      pid,
    });
  } finally {
    ctxBusy.value = false;
  }
}

async function ctxResume(pid: number) {
  const proc = processes.value.find((p) => p.pid === pid);
  const name = proc?.name || String(pid);
  const icon = proc?.icon ?? null;
  ctxBusy.value = true;
  try {
    await invoke("resume_process", { pid });
    await refresh();
    showResult({
      success: true,
      title: i18n.t("process.resumeSuccessTitle"),
      message: i18n.t("process.resumeSuccess", { pid }),
      icon,
      name,
      pid,
    });
  } catch (e) {
    showResult({
      success: false,
      title: i18n.t("process.resumeFailedTitle"),
      message: i18n.t("process.resumeFailed", { message: String(e) }),
      icon,
      name,
      pid,
    });
  } finally {
    ctxBusy.value = false;
  }
}

function ctxKill() {
  if (!ctxMenu.value) return;
  const pid = ctxMenu.value.pid;
  const proc = processes.value.find((p) => p.pid === pid);
  killTargetPid.value = pid;
  killTargetName.value = proc?.name || ctxMenu.value.name;
  killTargetIcon.value = proc?.icon ?? null;
  ctxMenu.value = null;
  killDialogOpen.value = true;
}

async function ctxPpl() {
  if (!ctxMenu.value) return;
  const pid = ctxMenu.value.pid;
  const name = ctxMenu.value.name;
  ctxMenu.value = null;
  pplPid.value = pid;
  pplError.value = "";
  pplContent.value = "";
  pplDialogOpen.value = true;
  try {
    const info = await invoke<string>("get_ppl_protection", { pid });
    pplContent.value = `${name} (PID: ${pid})\n${info}`;
  } catch (e) {
    pplError.value = i18n.t("process.pplFailed", { message: String(e) });
  }
}

async function ctxAdmin() {
  if (!ctxMenu.value) return;
  const pid = ctxMenu.value.pid;
  const proc = processes.value.find((p) => p.pid === pid);
  const name = proc?.name || ctxMenu.value.name;
  const icon = proc?.icon ?? null;
  ctxMenu.value = null;
  try {
    await invoke("restart_as_admin", { pid });
    showResult({
      success: true,
      title: i18n.t("process.adminSuccessTitle"),
      message: i18n.t("process.ctx.admin"),
      icon,
      name,
      pid,
    });
  } catch (e) {
    showResult({
      success: false,
      title: i18n.t("process.adminFailedTitle"),
      message: i18n.t("process.adminFailed", { message: String(e) }),
      icon,
      name,
      pid,
    });
  }
}

function onWinContextMenu(e: MouseEvent, w: WindowInfo) {
  e.preventDefault();
  winCtxMenu.value = { x: e.clientX, y: e.clientY, w };
}

function closeWinCtxMenu() {
  winCtxMenu.value = null;
}

function winResult(
  success: boolean,
  message: string,
  w: WindowInfo,
  titleKey: string
) {
  showResult({
    success,
    title: i18n.t(titleKey),
    message,
    icon: w.icon,
    name: w.title || w.exe_name || String(w.hwnd),
    pid: w.pid,
  });
}

async function wCtxCloseTask() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  if (selfPid !== null && w.pid === selfPid) {
    winResult(false, i18n.t("process.win.selfProtected"), w, "process.win.selfProtectedTitle");
    return;
  }
  winCtxBusy.value = true;
  try {
    await invoke("window_close_task", { hwnd: w.hwnd });
    await refresh();
    winResult(true, i18n.t("process.win.closeTaskSuccess"), w, "process.win.closeTaskSuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

async function wCtxKillProcess() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  if (selfPid !== null && w.pid === selfPid) {
    winResult(false, i18n.t("process.win.selfProtected"), w, "process.win.selfProtectedTitle");
    return;
  }
  winCtxBusy.value = true;
  try {
    await invoke("kill_process", { pid: w.pid });
    await refresh();
    winResult(true, i18n.t("process.win.killSuccess", { pid: w.pid }), w, "process.win.killSuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

async function wCtxClose() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  if (selfPid !== null && w.pid === selfPid) {
    winResult(false, i18n.t("process.win.selfProtected"), w, "process.win.selfProtectedTitle");
    return;
  }
  winCtxBusy.value = true;
  try {
    await invoke("window_destroy", { hwnd: w.hwnd });
    await refresh();
    winResult(true, i18n.t("process.win.closeSuccess"), w, "process.win.closeSuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

async function wCtxToggleTopmost() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  winCtxBusy.value = true;
  try {
    await invoke("window_set_topmost", { hwnd: w.hwnd, topmost: !w.is_topmost });
    await refresh();
    winResult(true, i18n.t(w.is_topmost ? "process.win.untopSuccess" : "process.win.topSuccess"), w, "process.win.topTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

async function wCtxCopyPath() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  try {
    const path = await invoke<string>("window_copy_path", { hwnd: w.hwnd });
    await navigator.clipboard.writeText(path);
    winResult(true, i18n.t("process.win.copySuccess"), w, "process.win.copySuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  }
}

async function wCtxToggleClickThrough() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  winCtxBusy.value = true;
  try {
    await invoke("window_set_click_through", { hwnd: w.hwnd, enabled: !w.is_click_through });
    await refresh();
    winResult(true, i18n.t(w.is_click_through ? "process.win.disableClickThrough" : "process.win.enableClickThrough"), w, "process.win.clickThroughTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

function wCtxSetOpacity() {
  if (!winCtxMenu.value) return;
  opacityTarget.value = winCtxMenu.value.w;
  opacityInput.value = winCtxMenu.value.w.opacity;
  winCtxMenu.value = null;
  opacityDialogOpen.value = true;
}

async function applyOpacity() {
  const w = opacityTarget.value;
  if (!w) return;
  opacityDialogOpen.value = false;
  winCtxBusy.value = true;
  try {
    await invoke("window_set_opacity", { hwnd: w.hwnd, opacity: opacityInput.value });
    await refresh();
    winResult(true, i18n.t("process.win.opacitySuccess", { value: opacityInput.value }), w, "process.win.opacitySuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
    opacityTarget.value = null;
  }
}

async function wCtxMinimize() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  winCtxBusy.value = true;
  try {
    await invoke("window_minimize", { hwnd: w.hwnd });
    await refresh();
    winResult(true, i18n.t("process.win.minimizeSuccess"), w, "process.win.minimizeSuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

async function wCtxRedraw() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  winCtxBusy.value = true;
  try {
    await invoke("window_redraw", { hwnd: w.hwnd });
    await refresh();
    winResult(true, i18n.t("process.win.redrawSuccess"), w, "process.win.redrawSuccessTitle");
  } catch (e) {
    winResult(false, i18n.t("process.win.actionFailed", { message: String(e) }), w, "process.win.failedTitle");
  } finally {
    winCtxBusy.value = false;
  }
}

function wCtxJumpToProcess() {
  if (!winCtxMenu.value) return;
  const w = winCtxMenu.value.w;
  winCtxMenu.value = null;
  activeTab.value = "process";
  selectedPid.value = w.pid;
  searchText.value = String(w.pid);
}

onMounted(() => {
  refreshSelfPid();
  refresh();
  autoRefreshTimer = window.setInterval(() => refresh(true), 3000);
});

onBeforeUnmount(() => {
  stopPickMode();
  if (autoRefreshTimer !== null) {
    clearInterval(autoRefreshTimer);
    autoRefreshTimer = null;
  }
});
</script>

<template>
  <PageShell title-key="process.title" subtitle-key="process.subtitle">
    <div class="proc-tabs">
      <button
        class="tab-item"
        :class="{ active: activeTab === 'process' }"
        type="button"
        @click="activeTab = 'process'"
      >
        {{ i18n.t("process.tab.process") }}
      </button>
      <button
        class="tab-item"
        :class="{ active: activeTab === 'window' }"
        type="button"
        @click="activeTab = 'window'"
      >
        {{ i18n.t("process.tab.window") }}
      </button>
    </div>

    <div class="proc-toolbar">
      <WinTextBox
        :Text="searchText"
        :PlaceholderText="i18n.t('process.search')"
        @update:Text="(v: string) => (searchText = v)"
        Style="flex:1;min-width:200px"
      />
      <WinButton
        :Content="i18n.t('process.refresh')"
        @click="refresh"
        :IsEnabled="!loading"
      />
      <WinButton
        :Content="pickMode ? i18n.t('process.pickWindowCancel') : i18n.t('process.pickWindow')"
        @click="pickMode ? stopPickMode() : startPickMode()"
        :IsEnabled="!loading"
        Appearance="Primary"
      />
      <WinButton
        :Content="i18n.t('process.kill')"
        @click="killSelected"
        :IsEnabled="!loading && selectedPid !== null"
      />
    </div>

    <WinInfoBar
      v-if="pickMode"
      :Title="i18n.t('process.pickWindowActive')"
      Severity="Informational"
      IsOpen
    />

    <WinInfoBar
      v-if="pickError"
      :Title="pickError"
      Severity="Error"
      IsOpen
    />

    <WinInfoBar
      v-if="error"
      :Title="error"
      Severity="Error"
      IsOpen
    />

    <div v-if="activeTab === 'process' && pickedPid !== null" class="picked-section">
      <div class="picked-header">
        <WinTextBlock :Text="i18n.t('process.pickedSection')" Style="font-weight:600;font-size:14px" />
        <WinButton
          :Content="i18n.t('process.pickedClear')"
          @click="clearPicked"
          Style="font-size:12px"
        />
      </div>
      <div v-if="pickedProcesses.length === 0" class="picked-empty">
        <WinTextBlock :Text="i18n.t('process.pickedEmpty')" Style="opacity:.6;font-size:13px" />
      </div>
      <div v-else class="proc-list">
        <div class="proc-list-head">
          <WinTextBlock :Text="''" Style="width:20px" />
          <WinTextBlock :Text="i18n.t('process.col.name')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.col.pid')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.col.cpu')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.memory')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.memoryPercent')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.status')" Style="font-weight:600" />
        </div>
        <div
          v-for="p in pickedProcesses"
          :key="`picked-${p.pid}`"
          class="proc-row"
          :class="{
            selected: selectedPid === p.pid,
            'is-self': p.is_self,
            'is-related': p.is_related,
          }"
          @click="selectRow(p.pid)"
          @contextmenu="onContextMenu($event, p)"
        >
          <img
            v-if="p.icon"
            :src="p.icon"
            class="proc-icon"
            alt=""
          />
          <AppIcon v-else name="app" class="proc-icon-placeholder" />
          <span class="proc-name-cell">
            <WinTextBlock :Text="p.name" Style="font-size:13px" />
            <span v-if="p.is_self" class="proc-self-badge">{{ i18n.t("process.selfBadge") }}</span>
            <span v-else-if="p.is_related" class="proc-related-badge">{{ i18n.t("process.relatedBadge") }}</span>
          </span>
          <WinTextBlock :Text="String(p.pid)" Style="font-size:13px;opacity:.8" Foreground="secondary" />
          <WinTextBlock :Text="`${p.cpu_usage.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
          <WinTextBlock :Text="formatBytes(p.memory)" Style="font-size:13px;text-align:right" />
          <WinTextBlock :Text="`${p.memory_percent.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
          <WinTextBlock :Text="statusLabel(p.status)" Style="font-size:13px" />
        </div>
      </div>
    </div>

    <template v-if="activeTab === 'process'">
      <WinTextBlock
        v-if="loading"
        :Text="i18n.t('process.loading')"
        Style="opacity:.6;padding:24px 0;text-align:center"
      />

      <div v-else-if="filteredProcesses.length > 0 || flatProcessTree.length > 0" class="proc-list">
        <div class="proc-list-head">
          <WinTextBlock :Text="''" Style="width:20px" />
          <WinTextBlock :Text="i18n.t('process.col.name')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.col.pid')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.col.cpu')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.memory')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.memoryPercent')" Style="font-weight:600;text-align:right" />
          <WinTextBlock :Text="i18n.t('process.col.status')" Style="font-weight:600" />
        </div>
        <template v-if="searchText.trim()">
          <div
            v-for="p in filteredProcesses"
            :key="p.pid"
            class="proc-row"
            :class="{
              selected: selectedPid === p.pid,
              'is-self': p.is_self,
              'is-related': p.is_related,
              'is-system': p.is_system,
            }"
            @click="selectRow(p.pid)"
            @contextmenu="onContextMenu($event, p)"
          >
            <img
              v-if="p.icon"
              :src="p.icon"
              class="proc-icon"
              alt=""
            />
            <AppIcon v-else name="app" class="proc-icon-placeholder" />
            <span class="proc-name-cell">
              <WinTextBlock :Text="p.name" Style="font-size:13px" />
              <span v-if="p.is_self" class="proc-self-badge">{{ i18n.t("process.selfBadge") }}</span>
              <span v-else-if="p.is_related" class="proc-related-badge">{{ i18n.t("process.relatedBadge") }}</span>
              <span v-else-if="p.is_system" class="proc-system-badge">{{ i18n.t("process.systemBadge") }}</span>
            </span>
            <WinTextBlock :Text="String(p.pid)" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="`${p.cpu_usage.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="formatBytes(p.memory)" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="`${p.memory_percent.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="statusLabel(p.status)" Style="font-size:13px" />
          </div>
        </template>
        <template v-else>
          <div
            v-for="node in flatProcessTree"
            :key="node.proc.pid"
            class="proc-row proc-tree-row"
            :class="{
              selected: selectedPid === node.proc.pid,
              'is-self': node.proc.is_self,
              'is-related': node.proc.is_related,
              'is-system': node.proc.is_system,
              'is-root': node.depth === 0,
              'is-child': node.depth > 0,
            }"
            :style="{ '--tree-depth': node.depth }"
            @click="selectRow(node.proc.pid)"
            @contextmenu="onContextMenu($event, node.proc)"
          >
            <span class="tree-branch" :class="{ 'is-last': node.isLast, 'is-root': node.depth === 0 }">
              <span v-if="node.depth === 0" class="tree-root-mark">●</span>
              <span v-else class="tree-prefix">{{ node.isLast ? '└─' : '├─' }}</span>
            </span>
            <img
              v-if="node.proc.icon"
              :src="node.proc.icon"
              class="proc-icon"
              alt=""
            />
            <AppIcon v-else name="app" class="proc-icon-placeholder" />
            <span class="proc-name-cell">
              <WinTextBlock :Text="node.proc.name" Style="font-size:13px" />
              <span v-if="node.proc.is_self" class="proc-self-badge">{{ i18n.t("process.selfBadge") }}</span>
              <span v-else-if="node.proc.is_related" class="proc-related-badge">{{ i18n.t("process.relatedBadge") }}</span>
              <span v-else-if="node.proc.is_system" class="proc-system-badge">{{ i18n.t("process.systemBadge") }}</span>
            </span>
            <WinTextBlock :Text="String(node.proc.pid)" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="`${node.proc.cpu_usage.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="formatBytes(node.proc.memory)" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="`${node.proc.memory_percent.toFixed(1)}%`" Style="font-size:13px;text-align:right" />
            <WinTextBlock :Text="statusLabel(node.proc.status)" Style="font-size:13px" />
          </div>
        </template>
      </div>

      <WinTextBlock
        v-else
        :Text="i18n.t('process.empty')"
        Style="opacity:.6;padding:24px 0;text-align:center"
      />

      <WinTextBlock
        v-if="processes.length > 0"
        :Text="i18n.t('process.totalCount', { count: processes.length })"
        Style="font-size:12px;opacity:.7;margin-top:12px"
        Foreground="secondary"
      />
    </template>

    <template v-else>
      <WinTextBlock
        v-if="loading"
        :Text="i18n.t('process.loading')"
        Style="opacity:.6;padding:24px 0;text-align:center"
      />

      <div v-else-if="filteredWindows.length > 0" class="proc-list win-list">
        <div class="proc-list-head win-list-head">
          <WinTextBlock :Text="''" Style="width:20px" />
          <WinTextBlock :Text="i18n.t('process.win.col.title')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.win.col.pid')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.win.col.exe')" Style="font-weight:600" />
          <WinTextBlock :Text="i18n.t('process.win.col.attrs')" Style="font-weight:600" />
        </div>
        <div
          v-for="w in filteredWindows"
          :key="w.hwnd"
          class="proc-row win-row"
          @contextmenu="onWinContextMenu($event, w)"
        >
          <img
            v-if="w.icon"
            :src="w.icon"
            class="proc-icon"
            alt=""
          />
          <AppIcon v-else name="app" class="proc-icon-placeholder" />
          <span class="win-title-cell">
            <WinTextBlock :Text="w.title || w.exe_name || String(w.hwnd)" Style="font-size:13px" />
          </span>
          <WinTextBlock :Text="String(w.pid)" Style="font-size:13px;opacity:.8" Foreground="secondary" />
          <WinTextBlock :Text="w.exe_name" Style="font-size:12px;opacity:.8" Foreground="secondary" />
          <span class="win-attrs">
            <span v-if="w.is_topmost" class="win-badge win-badge-top">{{ i18n.t("process.win.badge.topmost") }}</span>
            <span v-if="w.is_click_through" class="win-badge win-badge-click">{{ i18n.t("process.win.badge.clickThrough") }}</span>
            <span v-if="w.is_minimized" class="win-badge win-badge-min">{{ i18n.t("process.win.badge.minimized") }}</span>
            <span v-if="w.is_maximized" class="win-badge win-badge-max">{{ i18n.t("process.win.badge.maximized") }}</span>
            <span v-if="!w.is_visible" class="win-badge win-badge-hidden">{{ i18n.t("process.win.badge.hidden") }}</span>
            <span v-if="w.opacity < 255" class="win-badge win-badge-opacity">{{ i18n.t("process.win.badge.opacity", { value: w.opacity }) }}</span>
          </span>
        </div>
      </div>

      <WinTextBlock
        v-else
        :Text="i18n.t('process.win.empty')"
        Style="opacity:.6;padding:24px 0;text-align:center"
      />

      <WinTextBlock
        v-if="windows.length > 0"
        :Text="i18n.t('process.win.totalCount', { count: windows.length })"
        Style="font-size:12px;opacity:.7;margin-top:12px"
        Foreground="secondary"
      />
    </template>

    <WinContentDialog
      v-model:IsOpen="killDialogOpen"
      :Title="i18n.t('process.killConfirmTitle')"
      :PrimaryButtonText="i18n.t('process.killBtn')"
      :CloseButtonText="i18n.t('process.cancelBtn')"
      DefaultButton="Primary"
      @PrimaryButtonClick="confirmKill"
    >
      <div class="kill-dialog-body">
        <img
          v-if="killTargetIcon"
          :src="killTargetIcon"
          class="kill-dialog-icon"
          alt=""
        />
        <AppIcon v-else name="app" class="proc-icon-placeholder kill-dialog-icon" :size="32" />
        <div class="kill-dialog-text">
          <div class="kill-dialog-name-row">
            <span class="kill-dialog-name">{{ killTargetName }}</span>
            <span class="kill-dialog-pid">PID: {{ killTargetPid }}</span>
          </div>
          <div class="kill-dialog-hint">{{ i18n.t("process.killConfirmHint") }}</div>
        </div>
      </div>
    </WinContentDialog>

    <WinContentDialog
      v-model:IsOpen="resultDialogOpen"
      :Title="resultDialogTitle"
      :CloseButtonText="i18n.t('process.resultClose')"
      DefaultButton="Close"
    >
      <div class="kill-dialog-body">
        <img
          v-if="resultDialogIcon"
          :src="resultDialogIcon"
          class="kill-dialog-icon"
          alt=""
        />
        <AppIcon
          v-else
          name="app"
          class="proc-icon-placeholder kill-dialog-icon"
          :size="32"
          :color="resultDialogSeverity === 'error' ? '#c42b1c' : '#2ea043'"
        />
        <div class="kill-dialog-text">
          <div class="kill-dialog-name-row">
            <span class="kill-dialog-name">{{ resultDialogName }}</span>
            <span class="kill-dialog-pid">PID: {{ resultDialogPid }}</span>
          </div>
          <div class="kill-dialog-hint">{{ resultDialogMessage }}</div>
        </div>
      </div>
    </WinContentDialog>

    <WinContentDialog
      v-model:IsOpen="pplDialogOpen"
      :Title="i18n.t('process.pplTitle')"
      :CloseButtonText="i18n.t('process.cancelBtn')"
      DefaultButton="Close"
    >
      <div class="ppl-dialog-body">
        <div v-if="pplContent" class="ppl-content">{{ pplContent }}</div>
        <div v-if="pplError" class="ppl-error">{{ pplError }}</div>
      </div>
    </WinContentDialog>

    <Teleport to="body">
      <div
        v-if="ctxMenu"
        ref="ctxMenuEl"
        class="ctx-menu"
        :style="{ left: ctxMenuPos.left + 'px', top: ctxMenuPos.top + 'px' }"
        @click.stop
      >
        <div class="ctx-item" @click="ctxFreeze">{{ i18n.t("process.ctx.freeze") }}</div>
        <div class="ctx-item" @click="ctxResume(ctxMenu!.pid)">{{ i18n.t("process.ctx.resume") }}</div>
        <div class="ctx-item" @click="ctxKill">{{ i18n.t("process.ctx.kill") }}</div>
        <div class="ctx-item" @click="ctxPpl">{{ i18n.t("process.ctx.ppl") }}</div>
        <div class="ctx-item" @click="ctxAdmin">{{ i18n.t("process.ctx.admin") }}</div>
      </div>
      <div v-if="ctxMenu" class="ctx-overlay" @click="closeCtxMenu" @contextmenu.prevent="closeCtxMenu" />
    </Teleport>

    <Teleport to="body">
      <div
        v-if="winCtxMenu"
        ref="winCtxMenuEl"
        class="ctx-menu"
        :style="{ left: winCtxMenuPos.left + 'px', top: winCtxMenuPos.top + 'px' }"
        @click.stop
      >
        <div class="ctx-item" @click="wCtxCloseTask">{{ i18n.t("process.win.ctx.closeTask") }}</div>
        <div class="ctx-item" @click="wCtxKillProcess">{{ i18n.t("process.win.ctx.kill") }}</div>
        <div class="ctx-item" @click="wCtxClose">{{ i18n.t("process.win.ctx.close") }}</div>
        <div class="ctx-item" @click="wCtxToggleTopmost">{{ winCtxMenu!.w.is_topmost ? i18n.t("process.win.ctx.untop") : i18n.t("process.win.ctx.top") }}</div>
        <div class="ctx-item" @click="wCtxCopyPath">{{ i18n.t("process.win.ctx.copyPath") }}</div>
        <div class="ctx-item" @click="wCtxToggleClickThrough">{{ winCtxMenu!.w.is_click_through ? i18n.t("process.win.ctx.disableClickThrough") : i18n.t("process.win.ctx.enableClickThrough") }}</div>
        <div class="ctx-item" @click="wCtxSetOpacity">{{ i18n.t("process.win.ctx.setOpacity") }}</div>
        <div class="ctx-item" @click="wCtxMinimize">{{ i18n.t("process.win.ctx.minimize") }}</div>
        <div class="ctx-item" @click="wCtxRedraw">{{ i18n.t("process.win.ctx.redraw") }}</div>
        <div class="ctx-item" @click="wCtxJumpToProcess">{{ i18n.t("process.win.ctx.jumpToProcess") }}</div>
      </div>
      <div v-if="winCtxMenu" class="ctx-overlay" @click="closeWinCtxMenu" @contextmenu.prevent="closeWinCtxMenu" />
    </Teleport>

    <WinContentDialog
      v-model:IsOpen="opacityDialogOpen"
      :Title="i18n.t('process.win.opacityTitle')"
      :PrimaryButtonText="i18n.t('process.win.opacityApply')"
      :CloseButtonText="i18n.t('process.cancelBtn')"
      DefaultButton="Primary"
      @PrimaryButtonClick="applyOpacity"
    >
      <div class="opacity-dialog-body">
        <div class="opacity-dialog-row">
          <input
            type="range"
            min="0"
            max="255"
            v-model.number="opacityInput"
            class="opacity-slider"
            style="flex:1"
          />
          <input
            type="number"
            min="0"
            max="255"
            v-model.number="opacityInput"
            class="opacity-number"
          />
        </div>
      </div>
    </WinContentDialog>
  </PageShell>
</template>

<style scoped>
.proc-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}

html.theme-dark .proc-tabs {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.tab-item {
  appearance: none;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 8px 16px 10px;
  font-size: 13px;
  cursor: pointer;
  color: var(--TextFillColorSecondaryBrush, var(--text-secondary, inherit));
  transition: color 0.12s ease, border-color 0.12s ease;
}

.tab-item:hover {
  color: var(--TextFillColorPrimaryBrush, var(--text-primary, inherit));
}

.tab-item.active {
  color: var(--TextFillColorPrimaryBrush, var(--text-primary, inherit));
  border-bottom-color: var(--AccentButtonBackground, var(--accent-base, #005fb8));
}

.proc-toolbar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin: -4px -8px 16px;
  padding: 8px 8px;
  border-radius: 8px;
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(243, 243, 243, 0.85));
  backdrop-filter: blur(12px);
}

html.theme-dark .proc-toolbar {
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(32, 32, 32, 0.85));
}

.proc-list {
  border-radius: 8px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
  overflow: hidden;
}

html.theme-dark .proc-list {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.proc-list-head,
.proc-row {
  display: grid;
  grid-template-columns: 20px 1.6fr 0.8fr 0.8fr 1fr 0.8fr 0.8fr;
  gap: 12px;
  padding: 10px 16px;
  align-items: center;
}

.proc-icon {
  width: 16px;
  height: 16px;
  object-fit: contain;
}

.proc-icon-placeholder {
  width: 16px;
  height: 16px;
  color: var(--TextFillColorTertiaryBrush, var(--text-secondary, #8a8a8a));
  opacity: 0.7;
}

.proc-list-head {
  background: var(--SubtleFillColorTertiary, rgba(0, 0, 0, 0.04));
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}

html.theme-dark .proc-list-head {
  background: var(--SubtleFillColorTertiary, rgba(255, 255, 255, 0.06));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.proc-row {
  cursor: pointer;
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.04));
  transition: background 0.12s ease;
}

.proc-row:last-child {
  border-bottom: none;
}

.proc-row:hover {
  background: var(--SubtleFillColorTertiary, rgba(0, 0, 0, 0.04));
}

html.theme-dark .proc-row:hover {
  background: var(--SubtleFillColorTertiary, rgba(255, 255, 255, 0.06));
}

.proc-row.selected {
  background: var(--SubtleFillColorSecondary, rgba(0, 0, 0, 0.08));
}

html.theme-dark .proc-row.selected {
  background: var(--SubtleFillColorSecondary, rgba(255, 255, 255, 0.1));
}

.proc-tree-row {
  position: relative;
}

.proc-tree-row.is-child {
  padding-top: 2px;
  padding-bottom: 2px;
  padding-left: calc(var(--tree-depth, 0) * 18px + 28px);
  border-left: 3px solid color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 45%, transparent);
}

.proc-tree-row.is-root {
  padding-left: 30px;
  border-left: 3px solid color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 60%, transparent);
}

.tree-branch {
  position: absolute;
  left: calc(6px + var(--tree-depth, 0) * 18px);
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  align-items: center;
  justify-content: flex-start;
  width: 18px;
  font-family: Consolas, "Segoe UI", monospace;
  font-size: 13px;
  font-weight: 700;
  line-height: 1;
  color: var(--AccentButtonBackground, #005fb8);
  pointer-events: none;
}

.tree-branch.is-root {
  left: 4px;
  width: auto;
  justify-content: flex-start;
}

.tree-root-mark {
  color: var(--AccentButtonBackground, #005fb8);
  font-size: 12px;
  font-weight: 700;
}

.tree-prefix {
  white-space: nowrap;
}

.win-list-head,
.win-row {
  grid-template-columns: 20px 1.8fr 0.7fr 1fr 1.4fr !important;
}

.win-title-cell {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.win-attrs {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.win-badge {
  display: inline-block;
  padding: 1px 6px;
  font-size: 11px;
  border-radius: 4px;
  background: var(--SubtleFillColorSecondary, rgba(0, 0, 0, 0.08));
  color: var(--TextFillColorSecondaryBrush, var(--text-secondary, inherit));
}

html.theme-dark .win-badge {
  background: var(--SubtleFillColorSecondary, rgba(255, 255, 255, 0.1));
}

.win-badge-top {
  background: color-mix(in srgb, #005fb8 18%, transparent);
}

.win-badge-click {
  background: color-mix(in srgb, #c42b1c 16%, transparent);
}

.win-badge-min {
  background: color-mix(in srgb, #b08000 18%, transparent);
}

.win-badge-hidden {
  background: color-mix(in srgb, #6b6b6b 20%, transparent);
}

.opacity-dialog-body {
  padding: 4px 0;
}

.opacity-dialog-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.opacity-number {
  width: 64px;
  padding: 4px 8px;
  border-radius: 4px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.12));
  background: var(--ControlFillColorDefaultBrush, transparent);
  color: inherit;
  font-size: 13px;
}

.proc-name-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.proc-row.is-self {
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 7%, transparent);
  box-shadow: inset 3px 0 0 var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .proc-row.is-self {
  background: color-mix(in srgb, #4cc2ff 9%, transparent);
  box-shadow: inset 3px 0 0 #4cc2ff;
}

.proc-self-badge {
  flex-shrink: 0;
  font-size: 11px;
  line-height: 16px;
  padding: 0 6px;
  border-radius: 4px;
  color: #fff;
  background: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .proc-self-badge {
  color: #003a6b;
  background: #4cc2ff;
}

.proc-row.is-related {
  background: color-mix(in srgb, #107c10 6%, transparent);
  box-shadow: inset 3px 0 0 #107c10;
}

html.theme-dark .proc-row.is-related {
  background: color-mix(in srgb, #6fbb6f 8%, transparent);
  box-shadow: inset 3px 0 0 #6fbb6f;
}

.proc-related-badge {
  flex-shrink: 0;
  font-size: 11px;
  line-height: 16px;
  padding: 0 6px;
  border-radius: 4px;
  color: #fff;
  background: #107c10;
}

html.theme-dark .proc-related-badge {
  color: #0a2e0a;
  background: #6fbb6f;
}

.proc-row.is-system {
  background: color-mix(in srgb, #c19c00 6%, transparent);
  box-shadow: inset 3px 0 0 #c19c00;
}

html.theme-dark .proc-row.is-system {
  background: color-mix(in srgb, #ffd355 8%, transparent);
  box-shadow: inset 3px 0 0 #ffd355;
}

.proc-system-badge {
  flex-shrink: 0;
  font-size: 11px;
  line-height: 16px;
  padding: 0 6px;
  border-radius: 4px;
  color: #fff;
  background: #c19c00;
}

html.theme-dark .proc-system-badge {
  color: #3a2c00;
  background: #ffd355;
}

.kill-dialog-body {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 4px 0 8px;
}

.kill-dialog-icon {
  width: 32px;
  height: 32px;
  object-fit: contain;
  flex-shrink: 0;
}

.kill-dialog-text {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.kill-dialog-name-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  flex-wrap: wrap;
}

.kill-dialog-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary, inherit);
  word-break: break-all;
}

.kill-dialog-pid {
  font-size: 13px;
  color: var(--text-secondary, inherit);
  opacity: 0.85;
}

.kill-dialog-hint {
  font-size: 13px;
  color: var(--text-secondary, inherit);
  opacity: 0.9;
}

.picked-section {
  margin-bottom: 20px;
  padding: 12px 14px;
  border-radius: 8px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 3%, transparent);
}

html.theme-dark .picked-section {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
  background: color-mix(in srgb, #4cc2ff 4%, transparent);
}

.picked-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.picked-empty {
  padding: 12px 0;
  text-align: center;
}

.ppl-dialog-body {
  padding: 4px 0 8px;
  min-width: 320px;
  max-width: 480px;
}

.ppl-content {
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 13px;
  white-space: pre-wrap;
  word-break: break-all;
}

.ppl-error {
  color: #c00;
  font-size: 13px;
}

html.theme-dark .ppl-error {
  color: #ff6b6b;
}

.ctx-menu {
  position: fixed;
  z-index: 9999;
  min-width: 160px;
  padding: 4px;
  border-radius: 6px;
  background: var(--SolidBackgroundFillColorBaseBrush, #fff);
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.12));
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  transform-origin: top left;
  animation: ctxMenuIn 0.14s cubic-bezier(0.2, 0.8, 0.2, 1);
}

@keyframes ctxMenuIn {
  from {
    opacity: 0;
    transform: scale(0.9) translateY(-4px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

html.theme-dark .ctx-menu {
  background: var(--SolidBackgroundFillColorBaseBrush, #2b2b2b);
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.1));
}

.ctx-item {
  padding: 8px 14px;
  font-size: 13px;
  cursor: pointer;
  border-radius: 4px;
  color: var(--text-primary, inherit);
}

.ctx-item:hover {
  background: var(--SubtleFillColorSecondary, rgba(0, 0, 0, 0.08));
}

html.theme-dark .ctx-item:hover {
  background: var(--SubtleFillColorSecondary, rgba(255, 255, 255, 0.1));
}

.ctx-overlay {
  position: fixed;
  inset: 0;
  z-index: 9998;
  background: transparent;
}
</style>
