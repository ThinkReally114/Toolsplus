  1→<script setup lang="ts">
  2→import { ref, shallowRef, onMounted, onBeforeUnmount, watch } from "vue";
  3→import KeystrokesHud from "@/components/KeystrokesHud.vue";
  4→
  5→type KsPosition = "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center-bottom";
  6→type KsColorMode = "heat" | "rainbow" | "mono" | "custom";
  7→
  8→const hasTauri =
  9→  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
 10→
 11→const box = ref({ x: 0, y: 0, width: 0, height: 0, show: false });
 12→const label = shallowRef<string>("");
 13→const winTitle = shallowRef<string>("");
 14→const labelVisible = ref(false);
 15→
 16→let timer: ReturnType<typeof setInterval> | null = null;
 17→
 18→/** 绘制缓动状态 */
 19→let smooth = false;
 20→const easeMs = ref(180);
 21→
 22→/** Keystrokes HUD 配置（localStorage 持久化，与 VisualView 共享 key） */
 23→const ksEnabled = ref(localStorage.getItem("ks.enabled") !== "0");
 24→const ksPosition = ref<KsPosition>(
 25→  (localStorage.getItem("ks.position") as KsPosition) || "center-bottom"
 26→);
 27→const ksSize = ref<number>(Number(localStorage.getItem("ks.size")) || 48);
 28→const ksColorMode = ref<KsColorMode>(
 29→  (localStorage.getItem("ks.colorMode") as KsColorMode) || "heat"
 30→);
 31→const ksCustomColor = ref<string>(
 32→  localStorage.getItem("ks.color") || "#0078d4"
 33→);
 34→const ksShowStats = ref(localStorage.getItem("ks.showStats") !== "0");
 35→const ksShowSeq = ref(localStorage.getItem("ks.showSequence") !== "0");
 36→
 37→async function refresh() {
 38→  if (!hasTauri) return;
 39→  try {
 40→    const { invoke } = await import("@tauri-apps/api/core");
 41→    const w = await invoke<{
 42→      x: number;
 43→      y: number;
 44→      width: number;
 45→      height: number;
 46→      title: string;
 47→      exe_name: string;
 48→      pid: number;
 49→      is_self: boolean;
 50→      smooth: boolean;
 51→      ease: number;
 52→    }>("get_window_under_cursor");
 53→    if (!w || w.is_self) {
 54→      box.value.show = false;
 55→      labelVisible.value = false;
 56→      return;
 57→    }
 58→    smooth = w.smooth;
 59→    if (w.ease) easeMs.value = w.ease;
 60→    const next = { x: w.x, y: w.y, width: w.width, height: w.height };
 61→    box.value = { ...next, show: true };
 62→    winTitle.value = w.title;
 63→    label.value = w.exe_name ? `${w.exe_name} · PID ${w.pid}` : `PID ${w.pid}`;
 64→    labelVisible.value = true;
 65→  } catch {
 66→    box.value.show = false;
 67→    labelVisible.value = false;
 68→  }
 69→}
 70→
 71→onMounted(async () => {
 72→  localStorage.setItem("glass.pen", "1");
 73→  document.documentElement.style.background = "transparent";
 74→  document.body.style.background = "transparent";
 75→  const app = document.getElementById("app");
 76→  if (app) app.style.background = "transparent";
 77→  if (!hasTauri) return;
 78→  try {
 79→    const { getCurrentWindow } = await import("@tauri-apps/api/window");
 80→    await getCurrentWindow().setIgnoreCursorEvents(true);
 81→  } catch {
 82→    // ignore
 83→  }
 84→  try {
 85→    const { listen } = await import("@tauri-apps/api/event");
 86→    await listen<{
 87→      enabled: boolean;
 88→      position: KsPosition;
 89→      size: number;
 90→      colorMode: KsColorMode;
 91→      color?: string;
 92→      showStats: boolean;
 93→      showSequence: boolean;
 94→    }>("ks-config-changed", (e) => {
 95→      const p = e.payload;
 96→      ksEnabled.value = p.enabled;
 97→      ksPosition.value = p.position;
 98→      ksSize.value = p.size;
 99→      ksColorMode.value = p.colorMode;
100→      if (p.color) ksCustomColor.value = p.color;
101→      ksShowStats.value = p.showStats;
102→      ksShowSeq.value = p.showSequence;
103→      localStorage.setItem("ks.enabled", p.enabled ? "1" : "0");
104→      localStorage.setItem("ks.position", p.position);
105→      localStorage.setItem("ks.size", String(p.size));
106→      localStorage.setItem("ks.colorMode", p.colorMode);
107→      if (p.color) localStorage.setItem("ks.color", p.color);
108→      localStorage.setItem("ks.showStats", p.showStats ? "1" : "0");
109→      localStorage.setItem("ks.showSequence", p.showSequence ? "1" : "0");
110→    });
111→  } catch (e) {
112→    console.error("GlassView ks-config 监听失败", e);
113→  }
114→  refresh();
115→  timer = setInterval(refresh, 40);
116→});
117→
118→onBeforeUnmount(() => {
119→  if (timer) clearInterval(timer);
120→});
121→
122→watch(box, (b) => {
123→  if (b.show && (b.width <= 0 || b.height <= 0)) {
124→    box.value.show = false;
125→  }
126→});
127→</script>
128→
129→<template>
130→  <div class="hud" :class="{ smooth }" :style="{ '--hud-ease': easeMs + 'ms' }">
131→    <div class="hud-dim" :style="{
132→      transform: 'translate(' + box.x + 'px,' + box.y + 'px)',
133→      width: box.width + 'px',
134→      height: box.height + 'px',
135→      opacity: box.show ? 1 : 0,
136→    }" />
137→    <div class="hud-tag" v-show="labelVisible" :style="{
138→      transform: 'translate(' + box.x + 'px,' + (box.y - 48 > 0 ? box.y - 48 : box.y) + 'px)',
139→    }">
140→      <div class="hud-title">{{ winTitle }}</div>
141→      <div class="hud-sub">{{ label }}</div>
142→    </div>
143→    <div class="hud-box" v-show="box.show" :style="{
144→      transform: 'translate(' + box.x + 'px,' + box.y + 'px)',
145→      width: box.width + 'px',
146→      height: box.height + 'px',
147→    }" />
148→  </div>
149→
150→  <KeystrokesHud
151→    v-if="ksEnabled"
152→    :position="ksPosition"
153→    :size="ksSize"
154→    :colorMode="ksColorMode"
155→    :color="ksCustomColor"
156→    :showStats="ksShowStats"
157→    :showSequence="ksShowSeq"
158→  />
159→</template>
160→
161→<style scoped>
162→.hud {
163→  position: fixed;
164→  inset: 0;
165→  background: transparent;
166→  pointer-events: none;
167→  overflow: hidden;
168→}
169→
170→.hud-dim {
171→  position: fixed;
172→  left: 0;
173→  top: 0;
174→  pointer-events: none;
175→  z-index: 1;
176→  box-shadow: 0 0 0 200vmax rgba(0, 0, 0, 0.42);
177→  transition: opacity 0.22s ease;
178→}
179→
180→.hud.smooth .hud-dim {
181→  transition:
182→    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
183→    width var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
184→    height var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
185→    opacity 0.22s ease;
186→}
187→
188→.hud-box {
189→  position: fixed;
190→  left: 0;
191→  top: 0;
192→  z-index: 3;
193→  border: 2px solid rgba(0, 120, 212, 0.9);
194→  box-sizing: border-box;
195→  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25) inset;
196→}
197→
198→.hud.smooth .hud-box {
199→  transition:
200→    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
201→    width var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
202→    height var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1);
203→}
204→
205→.hud-tag {
206→  position: fixed;
207→  left: 0;
208→  top: 0;
209→  z-index: 4;
210→  background: rgba(0, 95, 184, 0.92);
211→  color: #fff;
212→  border-radius: 5px;
213→  padding: 6px 10px;
214→  white-space: nowrap;
215→  pointer-events: none;
216→  max-width: 70vw;
217→  display: flex;
218→  flex-direction: column;
219→  gap: 2px;
220→  will-change: transform;
221→}
222→
223→.hud.smooth .hud-tag {
224→  transition:
225→    transform var(--hud-ease, 180ms) cubic-bezier(0.22, 1, 0.36, 1),
226→    opacity var(--hud-ease, 180ms) ease;
227→}
228→
229→.hud-title {
230→  font-size: 12px;
231→  font-weight: 600;
232→  line-height: 1.2;
233→  overflow: hidden;
234→  text-overflow: ellipsis;
235→}
236→
237→.hud-sub {
238→  font-size: 11px;
239→  opacity: 0.9;
240→  line-height: 1.2;
241→  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
242→}
243→</style>