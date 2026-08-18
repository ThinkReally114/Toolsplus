<script setup lang="ts">
import { inject, ref, computed, onMounted } from "vue";
import PageShell from "@/components/PageShell.vue";
import WinButton from "@winui/components/WinButton.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinProgressBar from "@winui/components/WinProgressBar.vue";
import WinInfoBar from "@winui/components/WinInfoBar.vue";
import WinCheckBox from "@winui/components/WinCheckBox.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;

interface JunkItem {
  key: string;
  labelKey: string;
  descKey: string;
  size: number; // bytes
  path: string;
  selected: boolean;
}

const items = ref<JunkItem[]>([]);
const scanning = ref(false);
const cleaning = ref(false);
const error = ref("");
const lastCleaned = ref(0); // bytes

const hasTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

const totalSize = computed(() =>
  items.value.filter((i) => i.selected).reduce((s, i) => s + i.size, 0)
);

function formatSize(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 2)} ${units[i]}`;
}

async function scan() {
  error.value = "";
  scanning.value = true;
  items.value = [];
  try {
    if (hasTauri) {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = (await invoke("scan_junk")) as {
        key: string;
        path: string;
        size: number;
      }[];
      items.value = result.map((r) => ({
        ...r,
        labelKey: `diskclean.cat.${r.key}`,
        descKey: `diskclean.cat.${r.key}.desc`,
        selected: true,
      }));
    } else {
      // 无 Rust 后端时的演示数据
      const demo = [
        { key: "temp", path: "%TEMP%", size: 128 * 1024 * 1024 },
        { key: "windowsTemp", path: "C:\\Windows\\Temp", size: 64 * 1024 * 1024 },
        { key: "recycle", path: "C:\\$Recycle.Bin", size: 32 * 1024 * 1024 },
        { key: "prefetch", path: "C:\\Windows\\Prefetch", size: 8 * 1024 * 1024 },
        { key: "logs", path: "C:\\Windows\\Logs", size: 4 * 1024 * 1024 },
      ];
      await new Promise((r) => setTimeout(r, 600));
      items.value = demo.map((r) => ({
        ...r,
        labelKey: `diskclean.cat.${r.key}`,
        descKey: `diskclean.cat.${r.key}.desc`,
        selected: true,
      }));
    }
  } catch (e: any) {
    error.value = e?.message || String(e);
  } finally {
    scanning.value = false;
  }
}

async function clean() {
  error.value = "";
  cleaning.value = true;
  lastCleaned.value = 0;
  try {
    const targets = items.value
      .filter((i) => i.selected)
      .map((i) => ({ key: i.key, path: i.path, size: i.size }));
    if (targets.length === 0) return;

    if (hasTauri) {
      const { invoke } = await import("@tauri-apps/api/core");
      const freed = (await invoke("clean_junk", { targets })) as number;
      lastCleaned.value = freed;
      // 重新扫描
      await scan();
    } else {
      await new Promise((r) => setTimeout(r, 800));
      lastCleaned.value = totalSize.value;
      items.value = [];
    }
  } catch (e: any) {
    error.value = e?.message || String(e);
  } finally {
    cleaning.value = false;
  }
}

function toggleAll(v: boolean) {
  items.value.forEach((i) => (i.selected = v));
}
const allSelected = computed({
  get: () => items.value.length > 0 && items.value.every((i) => i.selected),
  set: toggleAll,
});
const someSelected = computed(() => items.value.some((i) => i.selected));

onMounted(scan);
</script>

<template>
  <PageShell title-key="nav.diskclean" subtitle-key="diskclean.subtitle">
    <WinInfoBar
      v-if="!hasTauri"
      :Title="i18n.t('diskclean.demoMode')"
      :Message="i18n.t('diskclean.demoModeDesc')"
      Severity="Informational"
      IsOpen
    />

    <div class="dc-toolbar">
      <WinButton :Content="i18n.t('diskclean.rescan')" @click="scan" :IsEnabled="!scanning && !cleaning" />
      <WinButton
        :Content="i18n.t('diskclean.clean')"
        Appearance="Primary"
        @click="clean"
        :IsEnabled="!scanning && !cleaning && someSelected"
      />
      <div class="dc-total">
        <WinTextBlock :Text="i18n.t('diskclean.selected', { count: items.filter(i => i.selected).length, size: formatSize(totalSize) })" Style="font-size:13px;opacity:.8" Foreground="secondary" />
        <WinTextBlock :Text="formatSize(totalSize)" Style="font-size:15px;font-weight:600" />
      </div>
    </div>

    <WinProgressBar v-if="scanning || cleaning" :IsIndeterminate="true" />

    <WinInfoBar
      v-if="error"
      :Title="error"
      Severity="Error"
      IsOpen
    />
    <WinInfoBar
      v-else-if="lastCleaned > 0"
      :Title="i18n.t('diskclean.cleaned', { size: formatSize(lastCleaned) })"
      Severity="Success"
      IsOpen
    />

    <div v-if="items.length > 0" class="dc-list">
      <div class="dc-list-head">
        <WinCheckBox
          :IsChecked="allSelected"
          @update:IsChecked="(v: boolean) => (allSelected = v)"
        />
        <WinTextBlock :Text="i18n.t('diskclean.col.name')" Style="font-weight:600" />
        <WinTextBlock :Text="i18n.t('diskclean.col.path')" Style="font-weight:600" />
        <WinTextBlock :Text="i18n.t('diskclean.col.size')" Style="font-weight:600;text-align:right" />
      </div>
      <div v-for="item in items" :key="item.key" class="dc-row">
        <WinCheckBox
          :IsChecked="item.selected"
          @update:IsChecked="(v: boolean) => (item.selected = v)"
        />
        <div class="dc-name">
          <WinTextBlock :Text="i18n.t(item.labelKey)" Style="font-weight:500" />
          <WinTextBlock :Text="i18n.t(item.descKey)" Style="font-size:12px;opacity:.7" Foreground="secondary" />
        </div>
        <WinTextBlock :Text="item.path" Style="font-size:12px;opacity:.8" Foreground="secondary" />
        <WinTextBlock :Text="formatSize(item.size)" Style="text-align:right" />
      </div>
    </div>
    <WinTextBlock
      v-else-if="!scanning"
      :Text="i18n.t('diskclean.empty')"
      Style="opacity:.6;padding:24px 0;text-align:center"
    />
  </PageShell>
</template>

<style scoped>
.dc-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.dc-total {
  margin-left: auto;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}
.dc-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
  border-radius: 8px;
  overflow: hidden;
}
.dc-list-head,
.dc-row {
  display: grid;
  grid-template-columns: 40px 1.4fr 1.6fr auto;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
}
.dc-list-head {
  background: var(--SubtleFillColorTertiary, rgba(0, 0, 0, 0.03));
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}
.dc-row {
  border-bottom: 1px solid var(--DividerStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}
.dc-row:last-child {
  border-bottom: none;
}
.dc-name {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
</style>
