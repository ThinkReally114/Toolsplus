  1→<script setup lang="ts">
  2→import { ref, computed, onMounted } from "vue";
  3→
  4→type Position = "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center-bottom";
  5→type ColorMode = "heat" | "rainbow" | "mono" | "custom";
  6→
  7→const props = withDefaults(
  8→  defineProps<{
  9→    position?: Position;
 10→    size?: number;
 11→    colorMode?: ColorMode;
 12→    color?: string;
 13→    showStats?: boolean;
 14→    showSequence?: boolean;
 15→  }>(),
 16→  {
 17→    position: "center-bottom",
 18→    size: 48,
 19→    colorMode: "heat",
 20→    color: "#0078d4",
 21→    showStats: true,
 22→    showSequence: true,
 23→  }
 24→);
 25→
 26→interface KeyState {
 27→  count: number;
 28→  last: number;
 29→  first: number;
 30→}
 31→
 32→const keys = ref<Record<string, KeyState>>({});
 33→
 34→let lastEventT = 0;
 35→
 36→const newKeys = ref<Set<string>>(new Set());
 37→
 38→function keyLabel(keyId: string): string {
 39→  if (keyId.startsWith("char:")) return keyId.slice(5);
 40→  const map: Record<string, string> = {
 41→    Control: "Ctrl",
 42→    Shift: "Shift",
 43→    Alt: "Alt",
 44→    Meta: "Win",
 45→    LMB: "LMB",
 46→    RMB: "RMB",
 47→  };
 48→  return map[keyId] ?? keyId;
 49→}
 50→
 51→function record(keyId: string, t: number) {
 52→  const isNew = !keys.value[keyId];
 53→  const cur = keys.value[keyId] || { count: 0, last: 0, first: t };
 54→  cur.count += 1;
 55→  cur.last = t;
 56→  keys.value[keyId] = cur;
 57→  lastEventT = t;
 58→  if (isNew) newKeys.value.add(keyId);
 59→  if (newKeys.value.size > 32) newKeys.value.clear();
 60→}
 61→
 62→const displayed = computed(() =>
 63→  Object.entries(keys.value)
 64→    .filter(([, st]) => st.count > 0)
 65→    .sort((a, b) => b[1].last - a[1].last)
 66→    .slice(0, 14)
 67→    .map(([id, st]) => ({ id, state: st, label: keyLabel(id) })
 68→  )
 69→);
 70→
 71→function positionStyle(pos: Position): Record<string, string> {
 72→  switch (pos) {
 73→    case "top-left":
 74→      return { left: "24px", top: "24px", right: "auto", bottom: "auto", transform: "none" };
 75→    case "top-right":
 76→      return { right: "24px", top: "24px", left: "auto", bottom: "auto", transform: "none" };
 77→    case "bottom-left":
 78→      return { left: "24px", bottom: "24px", right: "auto", top: "auto", transform: "none" };
 79→    case "bottom-right":
 80→      return { right: "24px", bottom: "24px", left: "auto", top: "auto", transform: "none" };
 81→    default:
 82→      return { left: "50%", bottom: "24px", transform: "translateX(-50%)" };
 83→  }
 84→}
 85→
 86→const hudStyle = computed(() => positionStyle(props.position));
 87→
 88→function heatColor(count: number): string {
 89→  const counts = Object.values(keys.value).map((k) => k.count);
 90→  const maxC = Math.max(10, ...counts);
 91→  const t = Math.min(1, count / maxC);
 92→  const stops: Array<[number, [number, number, number]]> = [
 93→    [0, [40, 96, 180]],
 94→    [0.25, [0, 168, 200]],
 95→    [0.5, [96, 190, 120]],
 96→    [0.75, [235, 190, 70]],
 97→    [1, [232, 90, 48]],
 98→  ];
 99→  for (let i = 0; i < stops.length - 1; i++) {
100→    if (t >= stops[i][0] && t <= stops[i + 1][0]) {
101→      const lo = t - stops[i][0];
102→      const span = stops[i + 1][0] - stops[i][0];
103→      const f = span > 0 ? lo / span : 0;
104→      const c0 = stops[i][1];
105→      const c1 = stops[i + 1][1];
106→      const r = Math.round(c0[0] + (c1[0] - c0[0]) * f);
107→      const g = Math.round(c0[1] + (c1[1] - c0[1]) * f);
108→      const b = Math.round(c0[2] + (c1[2] - c0[2]) * f);
109→      return `rgb(${r},${g},${b})`;
110→    }
111→  }
112→  return "rgb(232,90,48)";
113→}
114→
115→function rainbowColor(): string {
116→  const hue = (Date.now() / 8) % 360;
117→  return `hsl(${hue},85%,55%)`;
118→}
119→
120→function monoColor(active: boolean): string {
121→  return active ? "rgb(0,120,212)" : "rgba(128,128,128,0.5)";
122→}
123→
124→function keyColor(keyId: string): string {
125→  const k = keys.value[keyId];
126→  const count = k ? k.count : 0;
127→  const active = count > 0;
128→  if (props.colorMode === "rainbow") return active ? rainbowColor() : "rgba(128,128,128,0.35)";
129→  if (props.colorMode === "mono") return monoColor(active);
130→  if (props.colorMode === "custom") return props.color;
131→  return heatColor(count);
132→}
133→
134→onMounted(async () => {
135→  if (typeof (window as any).__TAURI_INTERNALS__ !== "undefined") {
136→    try {
137→      const { listen } = await import("@tauri-apps/api/event");
138→      await listen<{
139→        key: string;
140→        ctrlKey: boolean;
141→        shiftKey: boolean;
142→        altKey: boolean;
143→        metaKey: boolean;
144→        t: number;
145→      }>("ks-key-event", (e) => {
146→        handleKey(
147→          {
148→            key: e.payload.key,
149→            ctrlKey: e.payload.ctrlKey,
150→            shiftKey: e.payload.shiftKey,
151→            altKey: e.payload.altKey,
152→            metaKey: e.payload.metaKey,
153→          },
154→          e.payload.t
155→        );
156→      });
157→      await listen<{ button: number; t: number }>("ks-mouse-event", (e) => {
158→        const id = e.payload.button === 0 ? "LMB" : "RMB";
159→        record(id, e.payload.t);
160→      });
161→    } catch (err) {
162→      console.error("KeystrokesHud Tauri event setup failed", err);
163→    }
164→  }
165→});
166→
167→function handleKey(
168→  ev: {
169→    key: string;
170→    ctrlKey: boolean;
171→    shiftKey: boolean;
172→    altKey: boolean;
173→    metaKey: boolean;
174→  },
175→  t: number
176→) {
177→  if (ev.ctrlKey) record("Control", t);
178→  if (ev.shiftKey) record("Shift", t);
179→  if (ev.altKey) record("Alt", t);
180→  if (ev.metaKey) record("Meta", t);
181→  const k = ev.key;
182→  const kl = k ? k.toLowerCase() : "";
183→  if (kl === "w") record("KeyW", t);
184→  else if (kl === "a") record("KeyA", t);
185→  else if (kl === "s") record("KeyS", t);
186→  else if (kl === "d") record("KeyD", t);
187→  else if (k === "ArrowUp") record("ArrowUp", t);
188→  else if (k === "ArrowDown") record("ArrowDown", t);
189→  else if (k === "ArrowLeft") record("ArrowLeft", t);
190→  else if (k === "ArrowRight") record("ArrowRight", t);
191→  else if (k && k.length === 1) record("char:" + k.toUpperCase(), t);
192→  else if (k) record(k, t);
193→}
194→</script>
195→
196→<template>
197→  <div
198→    class="ks-hud"
199→    :class="props.colorMode"
200→    :style="hudStyle"
201→    v-if="props.showSequence"
202→  >
203→    <div class="ks-panel">
204→      <div class="ks-keys">
205→        <div
206→          v-for="item in displayed"
207→          :key="item.id"
208→          class="ks-key"
209→          :class="{ 'ks-key-new': newKeys.has(item.id) }"
210→          :style="{ width: size + 'px', height: size + 'px', background: keyColor(item.id) }"
211→        >
212→          <span class="ks-key-txt">{{ item.label }}</span>
213→          <span v-if="props.showStats && item.state.count > 1" class="ks-key-count">{{ item.state.count }}</span>
214→        </div>
215→      </div>
216→    </div>
217→  </div>
218→</template>
219→
220→<style scoped>
221→.ks-hud {
222→  position: fixed;
223→  z-index: 5;
224→  pointer-events: none;
225→  font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
226→  color: #e8eef7;
227→}
228→
229→.ks-panel {
230→  display: flex;
231→  flex-direction: column;
232→  align-items: center;
233→  gap: 10px;
234→  padding: 14px;
235→  background: rgba(16, 22, 32, 0.72);
236→  border: 1px solid rgba(255, 255, 255, 0.14);
237→  border-radius: 12px;
238→  backdrop-filter: blur(8px);
239→}
240→
241→.ks-keys {
242→  display: flex;
243→  flex-wrap: wrap;
244→  gap: 6px;
245→  justify-content: center;
246→  max-width: 80vw;
247→}
248→
249→.ks-key {
250→  position: relative;
251→  display: flex;
252→  align-items: center;
253→  justify-content: center;
254→  border-radius: 8px;
255→  font-weight: 600;
256→  font-size: 12px;
257→  color: #fff;
258→  margin-left: -8px;
259→}
260→
261→.ks-key:first-child {
262→  margin-left: 0;
263→}
264→
265→.ks-key-txt {
266→  position: relative;
267→  z-index: 1;
268→}
269→
270→.ks-key-count {
271→  position: absolute;
272→  top: -6px;
273→  right: -6px;
274→  min-width: 18px;
275→  height: 18px;
276→  padding: 0 4px;
277→  background: rgba(16, 22, 32, 0.92);
278→  border: 1px solid rgba(255, 255, 255, 0.28);
279→  border-radius: 9px;
280→  font-size: 11px;
281→  font-weight: 700;
282→  display: flex;
283→  align-items: center;
284→  justify-content: center;
285→  z-index: 2;
286→}
287→
288→.ks-key.ks-key-new {
289→  animation: ks-press 0.5s cubic-bezier(0.16, 1, 0.3, 1) both;
290→  will-change: transform, opacity;
291→}
292→
293→@keyframes ks-press {
294→  0% {
295→    transform: translateY(160px) scale(0.4);
296→    opacity: 0;
297→  }
298→  70% {
299→    transform: translateY(-6px) scale(1.35);
300→    opacity: 1;
301→  }
302→  100% {
303→    transform: translateY(0) scale(1);
304→    opacity: 1;
305→  }
306→}
307→</style>