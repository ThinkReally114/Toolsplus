<script setup lang="ts">
import { ref, inject, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import PageShell from "@/components/PageShell.vue";
import WinToggleSwitch from "@winui/components/WinToggleSwitch.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";
import AppIcon from "@/components/AppIcon.vue";

const i18n = inject<I18n>(i18nKey)!;
const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface SwitchDef {
  key: string;
  titleKey: string;
  descKey: string;
}

const switches: SwitchDef[] = [
  { key: "hover", titleKey: "visual.hover.title", descKey: "visual.hover.desc" },
  { key: "smooth", titleKey: "visual.smooth.title", descKey: "visual.smooth.desc" },
];

const hudShow = ref(false);
const states = ref<Record<string, boolean>>({
  hover: true,
  smooth: true,
});
const busy = ref<Record<string, boolean>>({ hover: false, smooth: false });

const easeMs = ref<number>(Number(localStorage.getItem("glass.ease.ms")) || 180);

async function setEase(ms: number) {
  easeMs.value = ms;
  localStorage.setItem("glass.ease.ms", String(ms));
  if (!hasTauri) return;
  try {
    await invoke("set_hud_ease", { ms });
  } catch (e) {
    console.error(e);
  }
}

async function refresh() {
  if (!hasTauri) return;
  try {
    const v = await invoke<{ hover: boolean; smooth: boolean; ease: number }>("get_visual_state");
    states.value.hover = v.hover;
    states.value.smooth = v.smooth;
    easeMs.value = v.ease;
  } catch {
    // ignore
  }
}

async function refreshHud() {
  if (!hasTauri) return;
  try {
    const { Window } = await import("@tauri-apps/api/window");
    const glass = await Window.getByLabel("glass");
    hudShow.value = glass ? await glass.isVisible() : false;
  } catch {
    hudShow.value = false;
  }
}

async function onToggle(key: string, enable: boolean) {
  if (!hasTauri) return;
  busy.value[key] = true;
  try {
    if (key === "hover") await invoke("set_hover_overlay", { enabled: enable });
    else if (key === "smooth") await invoke("set_hud_smooth", { enabled: enable });
    states.value[key] = enable;
  } catch (e) {
    console.error(e);
    await refresh();
  } finally {
    busy.value[key] = false;
  }
}

async function onHudToggle(show: boolean) {
  if (!hasTauri) return;
  try {
    const { Window } = await import("@tauri-apps/api/window");
    const glass = await Window.getByLabel("glass");
    if (!glass) return;
    if (show) await glass.show();
    else await glass.hide();
    hudShow.value = show;
  } catch (e) {
    console.error(e);
    await refreshHud();
  }
}

onMounted(() => {
  refresh();
  refreshHud();
});
</script>

<template>
  <WinScrollViewer VerticalScrollBarVisibility="Auto" VerticalScrollMode="Auto" class="visual-scroll">
    <PageShell :titleKey="'nav.visual'" :subtitleKey="'visual.subtitle'">
      <div class="visual-list">
        <div class="visual-card" :class="{ preview: hudShow }">
          <div class="visual-card-top">
            <div class="visual-preview-rows">
              <div class="visual-preview-row">
                <AppIcon name="hud" :size="18" />
                <WinTextBlock :Text="i18n.t('visual.hudShow.title')" Style="font-size:15px;font-weight:600" />
              </div>
              <WinTextBlock
                :Text="i18n.t('visual.hudShow.desc')"
                Style="font-size:12px;opacity:.7"
                Foreground="secondary"
              />
            </div>
            <WinToggleSwitch :IsOn="hudShow" :IsEnabled="hasTauri" @Toggled="onHudToggle($event?.IsOn ?? false)" />
          </div>
          <div class="visual-mini" v-show="hudShow">
            <span class="visual-mini-lbl">{{ i18n.t('visual.preview') }}</span>
            <span class="visual-mini-hint">{{ i18n.t('visual.preview.desc') }}</span>
          </div>
        </div>

        <div v-for="item in switches" :key="item.key" class="visual-card">
          <div class="visual-info">
            <WinTextBlock :Text="i18n.t(item.titleKey)" Style="font-size:15px;font-weight:600" />
            <WinTextBlock
              :Text="i18n.t(item.descKey)"
              Style="font-size:12px;opacity:.7"
              Foreground="secondary"
            />
          </div>
          <WinToggleSwitch
            :IsOn="states[item.key] ?? false"
            :IsEnabled="hasTauri && !busy[item.key]"
            @Toggled="onToggle(item.key, $event?.IsOn ?? false)"
          />
        </div>

        <div class="visual-card" v-show="states.smooth">
          <div class="visual-card-top">
            <div class="visual-info">
              <WinTextBlock :Text="i18n.t('visual.ease.title')" Style="font-size:15px;font-weight:600" />
              <WinTextBlock
                :Text="i18n.t('visual.ease.desc')"
                Style="font-size:12px;opacity:.7"
                Foreground="secondary"
              />
            </div>
            <WinTextBlock :Text="easeMs + 'ms'" Style="font-size:13px;font-weight:600" />
          </div>
          <input
            type="range"
            class="ease-slider"
            :min="60"
            :max="600"
            :step="20"
            :value="easeMs"
            :disabled="!hasTauri"
            @input="setEase(Number(($event.target as HTMLInputElement).value))"
          />
        </div>
      </div>
    </PageShell>
  </WinScrollViewer>
</template>

<style scoped>
.visual-scroll {
  height: 100%;
}
.visual-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}
.visual-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px;
  border-radius: 8px;
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(255, 255, 255, 0.7));
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
  transition: border-color 0.25s ease;
}
html.theme-dark .visual-card {
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}
.visual-card.preview {
  border-color: rgba(0, 120, 212, 0.55);
}
.visual-card-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.visual-preview-rows {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.visual-preview-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.visual-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.visual-mini {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(0, 120, 212, 0.08);
  font-size: 12px;
}
html.theme-dark .visual-mini {
  background: rgba(76, 194, 255, 0.1);
}
.visual-mini-lbl {
  font-weight: 600;
  color: #0078d4;
}
html.theme-dark .visual-mini-lbl {
  color: #4cc2ff;
}
.visual-mini-hint {
  opacity: 0.7;
}
.ease-slider {
  width: 100%;
  height: 4px;
  margin-top: 4px;
  border-radius: 4px;
  background: rgba(0, 120, 212, 0.25);
  outline: none;
  accent-color: #0078d4;
  cursor: pointer;
}
html.theme-dark .ease-slider {
  background: rgba(76, 194, 255, 0.25);
  accent-color: #4cc2ff;
}
</style>