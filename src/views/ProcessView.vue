<script setup lang="ts">
import { inject, ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
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
let iconToken = 0;

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

function selectRow(pid: number) {
  selectedPid.value = selectedPid.value === pid ? null : pid;
}

async function killSelected() {
  if (selectedPid.value === null) return;
  killTargetPid.value = selectedPid.value;
  const proc = processes.value.find((p) => p.pid === killTargetPid.value);
  killTargetName.value = proc?.name || String(killTargetPid.value);
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

onMounted(refresh);
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
        :Content="i18n.t('process.kill')"
        Appearance="Primary"
        @click="killSelected"
        :IsEnabled="!loading && selectedPid !== null"
      />
    </div>

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
        :class="{ selected: selectedPid === p.pid }"
        @click="selectRow(p.pid)"
      >
        <img
          v-if="p.icon"
          :src="p.icon"
          class="proc-icon"
          alt=""
        />
        <span v-else class="proc-icon-placeholder" />
        <WinTextBlock :Text="p.name" Style="font-size:13px" />
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
      :Content="i18n.t('process.killConfirm', { name: killTargetName, pid: killTargetPid || 0 })"
      :PrimaryButtonText="i18n.t('process.killBtn')"
      :CloseButtonText="i18n.t('process.cancelBtn')"
      DefaultButton="Primary"
      @PrimaryButtonClick="confirmKill"
    />
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
</style>
