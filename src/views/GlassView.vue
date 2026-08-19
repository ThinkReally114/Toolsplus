<script setup lang="ts">
import { ref, shallowRef, onMounted, onBeforeUnmount, watch } from "vue";

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const box = ref({ x: 0, y: 0, width: 0, height: 0, show: false });
const label = shallowRef<string>("");
const winTitle = shallowRef<string>("");
const labelVisible = ref(false);

let timer: ReturnType<typeof setInterval> | null = null;

/** 绘制缓动状态 */
let smooth = false;
const easeMs = ref(180);

async function refresh() {
  if (!hasTauri) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const w = await invoke<{
      x: number;
      y: number;
      width: number;
      height: number;
      title: string;
      exe_name: string;
      pid: number;
      is_self: boolean;
      smooth: boolean;
      ease: number;
    }>("get_window_under_cursor");
    if (!w || w.is_self) {
      box.value.show = false;
      labelVisible.value = false;
      return;
    }
    smooth = w.smooth;
    if (w.ease) easeMs.value = w.ease;
    const next = { x: w.x, y: w.y, width: w.width, height: w.height };
    box.value = { ...next, show: true };
    winTitle.value = w.title;
    label.value = w.exe_name ? `${w.exe_name} · PID ${w.pid}` : `PID ${w.pid}`;
    labelVisible.value = true;
  } catch {
    box.value.show = false;
    labelVisible.value = false;
  }
}

onMounted(async () => {
  localStorage.setItem("glass.pen", "1");
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  const app = document.getElementById("app");
  if (app) app.style.background = "transparent";
  if (!hasTauri) return;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch {
    // ignore
  }
  refresh();
  timer = setInterval(refresh, 40);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

watch(box, (b) => {
  if (b.show && (b.width <= 0 || b.height <= 0)) {
    box.value.show = false;
  }
});
</script>

<template>
  <div class="hud" :class="{ smooth }" :style="{ '--hud-ease': easeMs + 'ms' }">
    <div class="hud-dim" :style="{
      transform: 'translate(' + box.x + 'px,' + box.y + 'px)',
      width: box.width + 'px',
      height: box.height + 'px',
      opacity: box.show ? 1 : 0,
    }" />
    <div class="hud-tag" v-show="labelVisible" :style="{
      transform: 'translate(' + box.x + 'px,' + (box.y - 48 > 0 ? box.y - 48 : box.y) + 'px)',
    }">
      <div class="hud-title">{{ winTitle }}</div>
      <div class="hud-sub">{{ label }}</div>
    </div>
    <div class="hud-box" v-show="box.show" :style="{
      transform: 'translate(' + box.x + 'px,' + box.y + 'px)',
      width: box.width + 'px',
      height: box.height + 'px',
    }" />
  </div>
</template>

<style scoped>
.hud {
  position: fixed;
  inset: 0;
  background: transparent;
  pointer-events: none;
  overflow: hidden;
}

.hud-dim {
  position: fixed;
  left: 0;
  top: 0;
  pointer-events: none;
  z-index: 1;
  box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0.42);
  transition: opacity 0.22s ease;
}

.hud.smooth .hud-dim {
  transition:
    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    width var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    height var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    opacity 0.22s ease;
}

.hud-box {
  position: fixed;
  left: 0;
  top: 0;
  z-index: 3;
  border: 2px solid rgba(0, 120, 212, 0.9);
  box-sizing: border-box;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25) inset;
}

.hud.smooth .hud-box {
  transition:
    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    width var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    height var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1);
}

.hud-tag {
  position: fixed;
  left: 0;
  top: 0;
  z-index: 4;
  background: rgba(0, 95, 184, 0.92);
  color: #fff;
  border-radius: 5px;
  padding: 6px 10px;
  white-space: nowrap;
  pointer-events: none;
  max-width: 70vw;
  display: flex;
  flex-direction: column;
  gap: 2px;
  will-change: transform;
}

.hud.smooth .hud-tag {
  transition:
    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
    opacity var(--hud-ease, 180ms) ease;
}

.hud-title {
  font-size: 12px;
  font-weight: 600;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
}

.hud-sub {
  font-size: 11px;
  opacity: 0.9;
  line-height: 1.2;
  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
}
</style>
