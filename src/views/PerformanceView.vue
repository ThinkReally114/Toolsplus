<script setup lang="ts">
import { inject, ref, computed, onMounted, onUnmounted, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import PageShell from "@/components/PageShell.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinButton from "@winui/components/WinButton.vue";
import WinCheckBox from "@winui/components/WinCheckBox.vue";
import WinProgressBar from "@winui/components/WinProgressBar.vue";
import AppIcon from "@/components/AppIcon.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;

interface PerformanceStats {
  cpu_usage: number;
  cpu_per_core: number[];
  cpu_freq: number;
  cpu_cores: number;
  cpu_name: string;
  ram_total: number;
  ram_used: number;
  ram_usage: number;
  gpu_name: string;
  gpu_usage: number | null;
  gpu_vram_total: number | null;
  gpu_vram_used: number | null;
}

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

const stats = ref<PerformanceStats | null>(null);
const hardware = ref<HardwareInfo | null>(null);
const error = ref("");
const loading = ref(false);
const autoRefresh = ref(true);
let timer: number | null = null;

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const MAX_POINTS = 60;
const cpuHistory = ref<number[]>([]);
const ramHistory = ref<number[]>([]);
const gpuHistory = ref<number[]>([]);

function pushHistory(target: Ref<number[]>, v: number) {
  target.value.push(v);
  if (target.value.length > MAX_POINTS) target.value.shift();
}

function linePath(values: number[]): string {
  if (values.length < 2) return "";
  return values
    .map((v, i) => {
      const x = (i / (MAX_POINTS - 1)) * 100;
      const y = 100 - Math.min(100, Math.max(0, v));
      return `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

function areaPath(values: number[]): string {
  const line = linePath(values);
  if (!line) return "";
  return `${line} L100,100 L0,100 Z`;
}

const cpuLine = computed(() => linePath(cpuHistory.value));
const cpuArea = computed(() => areaPath(cpuHistory.value));
const ramLine = computed(() => linePath(ramHistory.value));
const ramArea = computed(() => areaPath(ramHistory.value));
const gpuLine = computed(() => linePath(gpuHistory.value));
const gpuArea = computed(() => areaPath(gpuHistory.value));

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024))
  );
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function formatFreq(mhz: number): string {
  if (mhz <= 0) return "—";
  if (mhz >= 1000) return `${(mhz / 1000).toFixed(2)} GHz`;
  return `${mhz} MHz`;
}

function coreCellStyle(v: number): Record<string, string> {
  const pct = Math.min(100, Math.max(0, v));
  const intensity = 0.1 + (pct / 100) * 0.85;
  return {
    background: `color-mix(in srgb, var(--AccentButtonBackground, #005fb8) ${(intensity * 100).toFixed(0)}%, transparent)`,
    color: pct > 55 ? "#fff" : "inherit",
  };
}

async function refresh() {
  if (!hasTauri) {
    error.value = "demo";
    return;
  }
  loading.value = true;
  error.value = "";
  try {
    const s = await invoke<PerformanceStats>("get_performance_stats");
    stats.value = s;
    pushHistory(cpuHistory, s.cpu_usage);
    pushHistory(ramHistory, s.ram_usage);
    if (s.gpu_usage !== null) pushHistory(gpuHistory, s.gpu_usage);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function startTimer() {
  stopTimer();
  if (autoRefresh.value) {
    timer = window.setInterval(refresh, 2000);
  }
}

function stopTimer() {
  if (timer !== null) {
    clearInterval(timer);
    timer = null;
  }
}

function onAutoRefreshChange(v: boolean) {
  autoRefresh.value = v;
  if (v) startTimer();
  else stopTimer();
}

onMounted(() => {
  refresh();
  loadHardware();
  startTimer();
});

onUnmounted(stopTimer);

async function loadHardware() {
  if (!hasTauri) return;
  try {
    hardware.value = await invoke<HardwareInfo>("get_hardware_info");
  } catch (e) {
    console.error(e);
  }
}
</script>

<template>
  <PageShell title-key="performance.title" subtitle-key="performance.subtitle">
    <div class="perf-toolbar">
      <WinButton
        :Content="i18n.t('performance.refresh')"
        @click="refresh"
        :IsEnabled="!loading"
      />
      <WinCheckBox
        :IsChecked="autoRefresh"
        @update:IsChecked="(v: boolean) => onAutoRefreshChange(v)"
      >
        <WinTextBlock :Text="i18n.t('performance.autoRefresh')" />
      </WinCheckBox>
    </div>

    <WinTextBlock
      v-if="error"
      :Text="error"
      Style="color:var(--system-error)"
    />

    <div class="perf-grid">
      <section class="perf-card">
        <div class="perf-card-header">
          <AppIcon name="cpu" :size="24" />
          <WinTextBlock :Text="i18n.t('performance.cpu')" Style="font-size:18px;font-weight:600" />
        </div>
        <div class="perf-card-body" v-if="stats">
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.cpu.usage')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="`${stats.cpu_usage.toFixed(1)}%`" Style="font-size:15px;font-weight:600" />
          </div>
          <WinProgressBar :Value="stats.cpu_usage" :Maximum="100" />
          <svg class="perf-chart" viewBox="0 0 100 100" preserveAspectRatio="none">
            <path class="chart-area" :d="cpuArea" />
            <path class="chart-line" :d="cpuLine" vector-effect="non-scaling-stroke" />
          </svg>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.cpu.logical')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
          </div>
          <div class="core-grid" v-if="stats.cpu_per_core.length > 0">
            <div
              v-for="(v, i) in stats.cpu_per_core"
              :key="i"
              class="core-cell"
              :style="coreCellStyle(v)"
            >
              {{ Math.round(v) }}
            </div>
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.cpu.freq')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatFreq(stats.cpu_freq)" Style="font-size:14px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.cpu.cores')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="String(stats.cpu_cores)" Style="font-size:14px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('home.hardware.cpu')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="stats.cpu_name" Style="font-size:13px" />
          </div>
        </div>
        <WinTextBlock
          v-else
          :Text="i18n.t('performance.loading')"
          Style="opacity:.6;padding:12px 0"
        />
      </section>

      <section class="perf-card">
        <div class="perf-card-header">
          <AppIcon name="ram" :size="24" />
          <WinTextBlock :Text="i18n.t('performance.memory')" Style="font-size:18px;font-weight:600" />
        </div>
        <div class="perf-card-body" v-if="stats">
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.memory.usage')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="`${stats.ram_usage.toFixed(1)}%`" Style="font-size:15px;font-weight:600" />
          </div>
          <WinProgressBar :Value="stats.ram_usage" :Maximum="100" />
          <svg class="perf-chart" viewBox="0 0 100 100" preserveAspectRatio="none">
            <path class="chart-area" :d="ramArea" />
            <path class="chart-line" :d="ramLine" vector-effect="non-scaling-stroke" />
          </svg>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.memory.used')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatBytes(stats.ram_used)" Style="font-size:14px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.memory.total')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatBytes(stats.ram_total)" Style="font-size:14px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.memory.free')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatBytes(stats.ram_total - stats.ram_used)" Style="font-size:14px" />
          </div>
        </div>
        <WinTextBlock
          v-else
          :Text="i18n.t('performance.loading')"
          Style="opacity:.6;padding:12px 0"
        />
      </section>

      <section class="perf-card">
        <div class="perf-card-header">
          <AppIcon name="gpu" :size="24" />
          <WinTextBlock :Text="i18n.t('performance.gpu')" Style="font-size:18px;font-weight:600" />
        </div>
        <div class="perf-card-body" v-if="stats">
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.gpu.name')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="stats.gpu_name" Style="font-size:13px" />
          </div>
          <div class="perf-row" v-if="stats.gpu_usage !== null">
            <WinTextBlock :Text="i18n.t('performance.gpu.usage')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="`${stats.gpu_usage.toFixed(1)}%`" Style="font-size:15px;font-weight:600" />
          </div>
          <WinProgressBar
            v-if="stats.gpu_usage !== null"
            :Value="stats.gpu_usage"
            :Maximum="100"
          />
          <svg
            v-if="stats.gpu_usage !== null && gpuLine"
            class="perf-chart"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
          >
            <path class="chart-area" :d="gpuArea" />
            <path class="chart-line" :d="gpuLine" vector-effect="non-scaling-stroke" />
          </svg>
          <div class="perf-row" v-if="stats.gpu_vram_used !== null">
            <WinTextBlock :Text="i18n.t('performance.gpu.vramUsed')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatBytes(stats.gpu_vram_used)" Style="font-size:14px" />
          </div>
          <div class="perf-row" v-if="stats.gpu_vram_total !== null">
            <WinTextBlock :Text="i18n.t('performance.gpu.vramTotal')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="formatBytes(stats.gpu_vram_total)" Style="font-size:14px" />
          </div>
          <WinTextBlock
            v-if="stats.gpu_usage === null && stats.gpu_vram_total === null"
            :Text="i18n.t('performance.notAvailable')"
            Style="opacity:.6;padding:8px 0"
          />
        </div>
        <WinTextBlock
          v-else
          :Text="i18n.t('performance.loading')"
          Style="opacity:.6;padding:12px 0"
        />
      </section>

      <section v-if="hardware" class="perf-card">
        <div class="perf-card-header">
          <AppIcon name="motherboard" :size="24" />
          <WinTextBlock :Text="i18n.t('performance.hardware')" Style="font-size:18px;font-weight:600" />
        </div>
        <div class="perf-card-body">
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.mb')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="hardware.motherboard" Style="font-size:13px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.ramSpeed')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="hardware.ram_speed" Style="font-size:14px;font-weight:600" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.ramBrand')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="hardware.ram_manufacturer" Style="font-size:13px" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.gpuVram')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="hardware.gpu_vram_total" Style="font-size:14px;font-weight:600" />
          </div>
          <div class="perf-row">
            <WinTextBlock :Text="i18n.t('performance.gpuDriver')" Style="font-size:13px;opacity:.8" Foreground="secondary" />
            <WinTextBlock :Text="hardware.gpu_driver" Style="font-size:13px" />
          </div>
        </div>
      </section>

      <section v-if="hardware?.disks?.length" class="perf-card">
        <div class="perf-card-header">
          <AppIcon name="diskclean" :size="24" />
          <WinTextBlock :Text="i18n.t('performance.disks')" Style="font-size:18px;font-weight:600" />
        </div>
        <div class="perf-card-body">
          <div v-for="(d, i) in hardware.disks" :key="i" class="perf-disk-row">
            <div class="perf-disk-name">{{ d.name || d.model }}</div>
            <div class="perf-disk-meta">
              <span v-if="d.interface">{{ i18n.t('performance.disk.if') }}: {{ d.interface }}</span>
              <span>{{ i18n.t('performance.disk.total') }}: {{ d.total }}</span>
              <span v-if="d.free !== '—'">{{ i18n.t('performance.disk.free') }}: {{ d.free }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  </PageShell>
</template>

<style scoped>
.perf-toolbar {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.perf-chart {
  display: block;
  width: 100%;
  height: 72px;
  box-sizing: border-box;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
  border-radius: 6px;
  background: var(--ControlFillColorDefaultBrush, rgba(0, 0, 0, 0.03));
  overflow: hidden;
}

html.theme-dark .perf-chart {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.06));
  background: var(--ControlFillColorDefaultBrush, rgba(255, 255, 255, 0.04));
}

.perf-chart .chart-line {
  fill: none;
  stroke: var(--AccentButtonBackground, #005fb8);
  stroke-width: 1.5;
  stroke-linejoin: round;
  stroke-linecap: round;
}

html.theme-dark .perf-chart .chart-line {
  stroke: #4cc2ff;
}

.perf-chart .chart-area {
  fill: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 14%, transparent);
}

html.theme-dark .perf-chart .chart-area {
  fill: color-mix(in srgb, #4cc2ff 14%, transparent);
}

.perf-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}

.perf-card {
  border-radius: 8px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.5));
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

html.theme-dark .perf-card {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.03));
}

.perf-card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .perf-card-header {
  color: var(--AccentButtonBackground, #4cc2ff);
}

.perf-card-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.perf-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.perf-disk-row {
  padding: 8px 0;
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}
.perf-disk-row:last-child {
  border-bottom: none;
}
.perf-disk-name {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 4px;
}
.perf-disk-meta {
  display: flex;
  gap: 16px;
  font-size: 12px;
  opacity: 0.7;
  flex-wrap: wrap;
}

.core-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(34px, 1fr));
  gap: 3px;
}

.core-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 26px;
  border-radius: 3px;
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}

html.theme-dark .core-cell {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.08));
}
</style>
