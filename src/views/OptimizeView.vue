<script setup lang="ts">
import { inject, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import PageShell from "@/components/PageShell.vue";
import WinToggleSwitch from "@winui/components/WinToggleSwitch.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinInfoBar from "@winui/components/WinInfoBar.vue";
import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;
const hasTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

interface OptItem {
  key: string;
  titleKey: string;
  descKey: string;
}
const items: OptItem[] = [
  { key: "smartscreen", titleKey: "opt.smartscreen.title", descKey: "opt.smartscreen.desc" },
  { key: "uac", titleKey: "opt.uac.title", descKey: "opt.uac.desc" },
  { key: "amsi", titleKey: "opt.amsi.title", descKey: "opt.amsi.desc" },
  { key: "stickykeys", titleKey: "opt.stickykeys.title", descKey: "opt.stickykeys.desc" },
];

const states = ref<Record<string, boolean>>({});
const busy = ref<Record<string, boolean>>({});
const error = ref("");

async function refresh() {
  if (!hasTauri) return;
  try {
    const arr = await invoke<{ key: string; enabled: boolean }[]>("optimize_states");
    const map: Record<string, boolean> = {};
    for (const s of arr) map[s.key] = s.enabled;
    states.value = map;
  } catch (e) {
    error.value = String(e);
  }
}

async function onToggle(key: string, enable: boolean) {
  busy.value[key] = true;
  error.value = "";
  try {
    await invoke("optimize_set", { key, enable });
    await refresh();
  } catch (e) {
    error.value = i18n.t("optimize.denied", { detail: String(e) });
    await refresh();
  } finally {
    busy.value[key] = false;
  }
}

onMounted(refresh);
</script>

<template>
  <WinScrollViewer class="optimize-scroll" VerticalScrollBarVisibility="Auto" VerticalScrollMode="Auto">
    <PageShell :titleKey="'nav.optimize'" :subtitleKey="'optimize.subtitle'">
      <WinInfoBar
        v-if="error"
        :Message="error"
        Severity="Error"
        :IsOpen="true"
        @ActionClick="error = ''"
      />

      <div class="opt-list">
        <div v-for="item in items" :key="item.key" class="opt-card">
          <div class="opt-info">
            <WinTextBlock :Text="i18n.t(item.titleKey)" Style="font-size:15px;font-weight:600" />
            <WinTextBlock
              :Text="i18n.t(item.descKey)"
              Style="font-size:12px;opacity:.7"
              Foreground="secondary"
            />
          </div>
          <WinToggleSwitch
            :IsOn="states[item.key] ?? false"
            :IsEnabled="!busy[item.key]"
            @Toggled="onToggle(item.key, $event?.IsOn ?? false)"
          />
        </div>
      </div>

      <WinTextBlock
        :Text="i18n.t('optimize.warn')"
        Style="font-size:12px;opacity:.6;margin-top:16px"
        Foreground="secondary"
      />
    </PageShell>
  </WinScrollViewer>
</template>

<style scoped>
.optimize-scroll {
  height: 100%;
}
.opt-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}
.opt-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-radius: 8px;
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(255, 255, 255, 0.7));
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}
html.theme-dark .opt-card {
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}
.opt-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-width: 0;
}
</style>
