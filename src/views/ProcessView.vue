<script setup lang="ts">
import { inject, ref, computed, onMounted, onBeforeUnmount } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { sendNotification } from "@tauri-apps/plugin-notification";
import PageShell from "@/components/PageShell.vue";
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

const processes = ref<ProcessInfo[]>([]);
const loading = ref(false);
const error = ref("");
const searchText = ref("");
const selectedPid = ref<number | null>(null);
const killMessage = ref("");
const killDialogOpen = ref(false);
const killTargetName = ref("");
const killTargetPid = ref<number | null>(null);
const killTargetIcon = ref<string | null>(null);
let iconToken = 0;

const pickMode = ref(false);
const pickedPid = ref<number | null>(null);
let pickTimer: number | null = null;
const pickError = ref("");

const ctxMenu = ref<{ x: number; y: number; pid: number; name: string } | null>(null);
const ctxBusy = ref(false);
const pplDialogOpen = ref(false);
const pplContent = ref("");
const pplPid = ref<number | null>(null);
const pplError = ref("");

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

async function refresh() {
  if (!hasTauri) {
    error.value = "demo";
    return;
  }
  loading.value = true;
  error.value = "";
  try {
    processes.value = await invoke<ProcessInfo[]>("list_processes");
    loadIcons();
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
  try {
    await invoke("kill_process", { pid });
    killMessage.value = i18n.t("process.killSuccess", { pid });
    selectedPid.value = null;
    await refresh();
  } catch (e) {
    killMessage.value = i18n.t("process.killFailed", { message: String(e) });
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
  ctxMenu.value = null;
  ctxBusy.value = true;
  try {
    await invoke("suspend_process", { pid });
    killMessage.value = i18n.t("process.freezeSuccess", { pid });
    await refresh();
  } catch (e) {
    killMessage.value = i18n.t("process.freezeFailed", { message: String(e) });
  } finally {
    ctxBusy.value = false;
  }
}

async function ctxResume(pid: number) {
  ctxBusy.value = true;
  try {
    await invoke("resume_process", { pid });
    killMessage.value = i18n.t("process.resumeSuccess", { pid });
    await refresh();
  } catch (e) {
    killMessage.value = i18n.t("process.resumeFailed", { message: String(e) });
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
  ctxMenu.value = null;
  try {
    await invoke("restart_as_admin", { pid });
    killMessage.value = i18n.t("process.ctx.admin");
  } catch (e) {
    killMessage.value = i18n.t("process.adminFailed", { message: String(e) });
  }
}

onMounted(refresh);

onBeforeUnmount(() => {
  stopPickMode();
});
</script>

<template>
  <PageShell title-key="process.title" subtitle-key="process.subtitle">
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
      v-if="killMessage"
      :Title="killMessage"
      Severity="Success"
      IsOpen
    />

    <WinInfoBar
      v-if="error"
      :Title="error"
      Severity="Error"
      IsOpen
    />

    <div v-if="pickedPid !== null" class="picked-section">
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
          <span v-else class="proc-icon-placeholder" />
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

    <WinTextBlock
      v-if="loading"
      :Text="i18n.t('process.loading')"
      Style="opacity:.6;padding:24px 0;text-align:center"
    />

    <div v-else-if="filteredProcesses.length > 0" class="proc-list">
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
        <span v-else class="proc-icon-placeholder" />
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
        <span v-else class="proc-icon-placeholder kill-dialog-icon" />
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
        class="ctx-menu"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
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
  </PageShell>
</template>

<style scoped>
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
  display: inline-block;
  width: 16px;
  height: 16px;
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
