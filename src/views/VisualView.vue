  1→<script setup lang="ts">
  2→import { ref, inject, onMounted } from "vue";
  3→import { invoke } from "@tauri-apps/api/core";
  4→import PageShell from "@/components/PageShell.vue";
  5→import WinToggleSwitch from "@winui/components/WinToggleSwitch.vue";
  6→import WinTextBlock from "@winui/components/WinTextBlock.vue";
  7→import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
  8→import { i18nKey, type I18n } from "@winui/components/i18n/index";
  9→import AppIcon from "@/components/AppIcon.vue";
 10→
 11→type KsPosition = "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center-bottom";
 12→type KsColorMode = "heat" | "rainbow" | "mono" | "custom";
 13→
 14→const i18n = inject<I18n>(i18nKey)!;
 15→const hasTauri =
 16→  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
 17→
 18→interface SwitchDef {
 19→  key: string;
 20→  titleKey: string;
 21→  descKey: string;
 22→}
 23→
 24→const switches: SwitchDef[] = [
 25→  { key: "hover", titleKey: "visual.hover.title", descKey: "visual.hover.desc" },
 26→  { key: "smooth", titleKey: "visual.smooth.title", descKey: "visual.smooth.desc" },
 27→];
 28→
 29→const hudShow = ref(false);
 30→const states = ref<Record<string, boolean>>({
 31→  hover: true,
 32→  smooth: true,
 33→});
 34→const busy = ref<Record<string, boolean>>({ hover: false, smooth: false });
 35→
 36→const easeMs = ref<number>(Number(localStorage.getItem("glass.ease.ms")) || 180);
 37→
 38→const ksEnabled = ref(localStorage.getItem("ks.enabled") !== "0");
 39→const ksPosition = ref<KsPosition>(
 40→  (localStorage.getItem("ks.position") as KsPosition) || "center-bottom"
 41→);
 42→const ksSize = ref<number>(Number(localStorage.getItem("ks.size")) || 48);
 43→const ksColorMode = ref<KsColorMode>(
 44→  (localStorage.getItem("ks.colorMode") as KsColorMode) || "heat"
 45→);
 46→const ksShowStats = ref(localStorage.getItem("ks.showStats") !== "0");
 47→const ksShowSeq = ref(localStorage.getItem("ks.showSequence") !== "0");
 48→const ksCustomColor = ref(localStorage.getItem("ks.color") || "#0078d4");
 49→
 50→function saveKs(key: string, value: string) {
 51→  localStorage.setItem(key, value);
 52→  void notifyKsConfig();
 53→}
 54→
 55→type KsConfigPayload = {
 56→  enabled: boolean;
 57→  position: KsPosition;
 58→  size: number;
 59→  colorMode: KsColorMode;
 60→  color?: string;
 61→  showStats: boolean;
 62→  showSequence: boolean;
 63→};
 64→
 65→async function notifyKsConfig() {
 66→  const cfg: KsConfigPayload = {
 67→    enabled: ksEnabled.value,
 68→    position: ksPosition.value,
 69→    size: ksSize.value,
 70→    colorMode: ksColorMode.value,
 71→    color: ksCustomColor.value,
 72→    showStats: ksShowStats.value,
 73→    showSequence: ksShowSeq.value,
 74→  };
 75→  if (!hasTauri) return;
 76→  try {
 77→    const { emitTo } = await import("@tauri-apps/api/event");
 78→    const { Window } = await import("@tauri-apps/api/window");
 79→    await emitTo("glass", "ks-config-changed", cfg);
 80→    await invoke("set_ks_enabled", { enabled: cfg.enabled });
 81→    if (cfg.enabled) {
 82→      const glass = await Window.getByLabel("glass");
 83→      if (glass && !(await glass.isVisible())) {
 84→        await glass.show();
 85→        hudShow.value = true;
 86→      }
 87→    }
 88→  } catch (e) {
 89→    console.error("ks-config 同步失败", e);
 90→  }
 91→}
 92→
 93→function setKsPosition(v: KsPosition) {
 94→  ksPosition.value = v;
 95→  saveKs("ks.position", v);
 96→}
 97→
 98→function posLabel(v: KsPosition): string {
 99→  const map: Record<KsPosition, string> = {
100→    "top-left": "左上",
101→    "top-right": "右上",
102→    "bottom-left": "左下",
103→    "bottom-right": "右下",
104→    "center-bottom": "中下",
105→  };
106→  return map[v] ?? v;
107→}
108→
109→function setKsSize(v: number) {
110→  ksSize.value = v;
111→  saveKs("ks.size", String(v));
112→}
113→
114→function setKsColorMode(v: KsColorMode) {
115→  ksColorMode.value = v;
116→  saveKs("ks.colorMode", v);
117→}
118→
119→function setKsCustomColor(v: string) {
120→  ksCustomColor.value = v;
121→  saveKs("ks.color", v);
122→}
123→
124→function setKsEnabled(v: boolean) {
125→  ksEnabled.value = v;
126→  saveKs("ks.enabled", v ? "1" : "0");
127→}
128→
129→function setKsShowStats(v: boolean) {
130→  ksShowStats.value = v;
131→  saveKs("ks.showStats", v ? "1" : "0");
132→}
133→
134→function setKsShowSeq(v: boolean) {
135→  ksShowSeq.value = v;
136→  saveKs("ks.showSequence", v ? "1" : "0");
137→}
138→
139→const ksToggles = [
140→  {
141→    key: "stats",
142→    titleKey: "visual.keystrokes.stats.title",
143→    descKey: "visual.keystrokes.stats.desc",
144→  },
145→  {
146→    key: "seq",
147→    titleKey: "visual.keystrokes.sequence.title",
148→    descKey: "visual.keystrokes.sequence.desc",
149→  },
150→];
151→
152→async function setEase(ms: number) {
153→  easeMs.value = ms;
154→  localStorage.setItem("glass.ease.ms", String(ms));
155→  if (!hasTauri) return;
156→  try {
157→    await invoke("set_hud_ease", { ms });
158→  } catch (e) {
159→    console.error(e);
160→  }
161→}
162→
163→async function refresh() {
164→  if (!hasTauri) return;
165→  try {
166→    const v = await invoke<{ hover: boolean; smooth: boolean; ease: number }>("get_visual_state");
167→    states.value.hover = v.hover;
168→    states.value.smooth = v.smooth;
169→    easeMs.value = v.ease;
170→  } catch {
171→    // ignore
172→  }
173→}
174→
175→async function refreshHud() {
176→  if (!hasTauri) return;
177→  try {
178→    const { Window } = await import("@tauri-apps/api/window");
179→    const glass = await Window.getByLabel("glass");
180→    hudShow.value = glass ? await glass.isVisible() : false;
181→  } catch {
182→    hudShow.value = false;
183→  }
184→}
185→
186→async function onToggle(key: string, enable: boolean) {
187→  if (!hasTauri) return;
188→  busy.value[key] = true;
189→  try {
190→    if (key === "hover") await invoke("set_hover_overlay", { enabled: enable });
191→    else if (key === "smooth") await invoke("set_hud_smooth", { enabled: enable });
192→    states.value[key] = enable;
193→  } catch (e) {
194→    console.error(e);
195→    await refresh();
196→  } finally {
197→    busy.value[key] = false;
198→  }
199→}
200→
201→async function onHudToggle(show: boolean) {
202→  if (!hasTauri) return;
203→  try {
204→    const { Window } = await import("@tauri-apps/api/window");
205→    const glass = await Window.getByLabel("glass");
206→    if (!glass) return;
207→    if (show) await glass.show();
208→    else await glass.hide();
209→    hudShow.value = show;
210→  } catch (e) {
211→    console.error(e);
212→    await refreshHud();
213→  }
214→}
215→
216→onMounted(() => {
217→  refresh();
218→  refreshHud();
219→});
220→</script>
221→
222→<template>
223→  <WinScrollViewer VerticalScrollBarVisibility="Auto" VerticalScrollMode="Auto" class="visual-scroll">
224→    <PageShell :titleKey="'nav.visual'" :subtitleKey="'visual.subtitle'">
225→      <div class="visual-list">
226→        <div class="visual-card" :class="{ preview: hudShow }">
227→          <div class="visual-card-top">
228→            <div class="visual-preview-rows">
229→              <div class="visual-preview-row">
230→                <AppIcon name="hud" :size="18" />
231→                <WinTextBlock :Text="i18n.t('visual.hudShow.title')" Style="font-size:15px;font-weight:600" />
232→              </div>
233→              <WinTextBlock
234→                :Text="i18n.t('visual.hudShow.desc')"
235→                Style="font-size:12px;opacity:.7"
236→                Foreground="secondary"
237→              />
238→            </div>
239→            <WinToggleSwitch :IsOn="hudShow" :IsEnabled="hasTauri" @Toggled="onHudToggle($event?.IsOn ?? false)" />
240→          </div>
241→          <div class="visual-mini" v-show="hudShow">
242→            <span class="visual-mini-lbl">{{ i18n.t('visual.preview') }}</span>
243→            <span class="visual-mini-hint">{{ i18n.t('visual.preview.desc') }}</span>
244→          </div>
245→        </div>
246→
247→        <div v-for="item in switches" :key="item.key" class="visual-card">
248→          <div class="visual-info">
249→            <WinTextBlock :Text="i18n.t(item.titleKey)" Style="font-size:15px;font-weight:600" />
250→            <WinTextBlock
251→              :Text="i18n.t(item.descKey)"
252→              Style="font-size:12px;opacity:.7"
253→              Foreground="secondary"
254→            />
255→          </div>
256→          <WinToggleSwitch
257→            :IsOn="states[item.key] ?? false"
258→            :IsEnabled="hasTauri && !busy[item.key]"
259→            @Toggled="onToggle(item.key, $event?.IsOn ?? false)"
260→          />
261→        </div>
262→
263→        <div class="visual-card" v-show="states.smooth">
264→          <div class="visual-card-top">
265→            <div class="visual-info">
266→              <WinTextBlock :Text="i18n.t('visual.ease.title')" Style="font-size:15px;font-weight:600" />
267→              <WinTextBlock
268→                :Text="i18n.t('visual.ease.desc')"
269→                Style="font-size:12px;opacity:.7"
270→                Foreground="secondary"
271→              />
272→            </div>
273→            <WinTextBlock :Text="easeMs + 'ms'" Style="font-size:13px;font-weight:600" />
274→          </div>
275→          <input
276→            type="range"
277→            class="ease-slider"
278→            :min="60"
279→            :max="600"
280→            :step="20"
281→            :value="easeMs"
282→            :disabled="!hasTauri"
283→            @input="setEase(Number(($event.target as HTMLInputElement).value))"
284→          />
285→        </div>
286→
287→        <div class="visual-card">
288→          <div class="visual-card-top">
289→            <div class="visual-info">
290→              <WinTextBlock :Text="i18n.t('visual.keystrokes.title')" Style="font-size:15px;font-weight:600" />
291→              <WinTextBlock
292→                :Text="i18n.t('visual.keystrokes.desc')"
293→                Style="font-size:12px;opacity:.7"
294→                Foreground="secondary"
295→              />
296→            </div>
297→            <WinToggleSwitch :IsOn="ksEnabled" :IsEnabled="hasTauri" @Toggled="setKsEnabled($event?.IsOn ?? false)" />
298→          </div>
299→        </div>
300→
301→        <template v-if="ksEnabled">
302→          <div class="visual-card">
303→            <div class="visual-info">
304→              <WinTextBlock :Text="i18n.t('visual.keystrokes.position.title')" Style="font-size:15px;font-weight:600" />
305→              <WinTextBlock
306→                :Text="i18n.t('visual.keystrokes.position.desc')"
307→                Style="font-size:12px;opacity:.7"
308→                Foreground="secondary"
309→              />
310→            </div>
311→            <div class="ks-seg">
312→              <button
313→                v-for="opt in ['top-left', 'top-right', 'bottom-left', 'bottom-right', 'center-bottom']"
314→                :key="opt"
315→                type="button"
316→                class="ks-seg-btn"
317→                :class="{ active: ksPosition === opt }"
318→                @click="setKsPosition(opt as KsPosition)"
319→              >
320→                {{ posLabel(opt as KsPosition) }}
321→              </button>
322→            </div>
323→          </div>
324→
325→          <div class="visual-card">
326→            <div class="visual-card-top">
327→              <div class="visual-info">
328→                <WinTextBlock :Text="i18n.t('visual.keystrokes.size.title')" Style="font-size:15px;font-weight:600" />
329→                <WinTextBlock
330→                  :Text="i18n.t('visual.keystrokes.size.desc')"
331→                  Style="font-size:12px;opacity:.7"
332→                  Foreground="secondary"
333→                />
334→              </div>
335→              <WinTextBlock :Text="ksSize + 'px'" Style="font-size:13px;font-weight:600" />
336→            </div>
337→            <input
338→              type="range"
339→              class="ease-slider"
340→              :min="28"
341→              :max="80"
342→              :step="2"
343→              :value="ksSize"
344→              :disabled="!hasTauri"
345→              @input="setKsSize(Number(($event.target as HTMLInputElement).value))"
346→            />
347→          </div>
348→
349→          <div class="visual-card">
350→            <div class="visual-info">
351→              <WinTextBlock :Text="i18n.t('visual.keystrokes.color.title')" Style="font-size:15px;font-weight:600" />
352→              <WinTextBlock
353→                :Text="i18n.t('visual.keystrokes.color.desc')"
354→                Style="font-size:12px;opacity:.7"
355→                Foreground="secondary"
356→              />
357→            </div>
358→            <div class="ks-color-controls">
359→              <div class="ks-seg">
360→                <button
361→                  v-for="opt in ['heat', 'rainbow', 'mono', 'custom']"
362→                  :key="opt"
363→                  type="button"
364→                  class="ks-seg-btn"
365→                  :class="{ active: ksColorMode === opt }"
366→                  @click="setKsColorMode(opt as KsColorMode)"
367→                >
368→                  {{ opt }}
369→                </button>
370→              </div>
371→              <input
372→                v-if="ksColorMode === 'custom'"
373→                type="color"
374→                class="ks-color-input"
375→                :value="ksCustomColor"
376→                :disabled="!hasTauri"
377→                @input="setKsCustomColor(($event.target as HTMLInputElement).value)"
378→              />
379→            </div>
380→          </div>
381→
382→          <div v-for="item in ksToggles" :key="item.key" class="visual-card">
383→            <div class="visual-info">
384→              <WinTextBlock :Text="i18n.t(item.titleKey)" Style="font-size:15px;font-weight:600" />
385→              <WinTextBlock
386→                :Text="i18n.t(item.descKey)"
387→                Style="font-size:12px;opacity:.7"
388→                Foreground="secondary"
389→              />
390→            </div>
391→            <WinToggleSwitch
392→              :IsOn="item.key === 'stats' ? ksShowStats : ksShowSeq"
393→              :IsEnabled="hasTauri"
394→              @Toggled="item.key === 'stats' ? setKsShowStats($event?.IsOn ?? false) : setKsShowSeq($event?.IsOn ?? false)"
395→            />
396→          </div>
397→        </template>
398→      </div>
399→    </PageShell>
400→  </WinScrollViewer>
401→</template>
402→
403→<style scoped>
404→.visual-scroll {
405→  height: 100%;
406→}
407→.visual-list {
408→  display: flex;
409→  flex-direction: column;
410→  gap: 10px;
411→  margin-top: 12px;
412→}
413→.visual-card {
414→  display: flex;
415→  flex-direction: column;
416→  gap: 10px;
417→  padding: 14px 16px;
418→  border-radius: 8px;
419→  background: var(--CardBackgroundFillColorDefaultBrush, rgba(255, 255, 255, 0.7));
420→  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
421→  transition: border-color 0.25s ease;
422→}
423→html.theme-dark .visual-card {
424→  background: var(--CardBackgroundFillColorDefaultBrush, rgba(32, 32, 32, 0.6));
425→  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
426→}
427→.visual-card.preview {
428→  border-color: rgba(0, 120, 212, 0.55);
429→}
430→.visual-card-top {
431→  display: flex;
432→  align-items: center;
433→  justify-content: space-between;
434→  gap: 16px;
435→}
436→.visual-preview-rows {
437→  display: flex;
438→  flex-direction: column;
439→  gap: 8px;
440→}
441→.visual-preview-row {
442→  display: flex;
443→  align-items: center;
444→  gap: 8px;
445→}
446→.visual-info {
447→  display: flex;
448→  flex-direction: column;
449→  gap: 4px;
450→}
451→.visual-mini {
452→  display: flex;
453→  align-items: center;
454→  gap: 8px;
455→  padding: 10px 12px;
456→  border-radius: 6px;
457→  background: rgba(0, 120, 212, 0.08);
458→  font-size: 12px;
459→}
460→html.theme-dark .visual-mini {
461→  background: rgba(76, 194, 255, 0.1);
462→}
463→.visual-mini-lbl {
464→  font-weight: 600;
465→  color: #0078d4;
466→}
467→html.theme-dark .visual-mini-lbl {
468→  color: #4cc2ff;
469→}
470→.visual-mini-hint {
471→  opacity: 0.7;
472→}
473→.ease-slider {
474→  width: 100%;
475→  height: 4px;
476→  margin-top: 4px;
477→  border-radius: 4px;
478→  background: rgba(0, 120, 212, 0.25);
479→  outline: none;
480→  accent-color: #0078d4;
481→  cursor: pointer;
482→}
483→html.theme-dark .ease-slider {
484→  background: rgba(76, 194, 255, 0.25);
485→  accent-color: #4cc2ff;
486→}
487→.ks-seg {
488→  display: flex;
489→  gap: 6px;
490→  flex-wrap: wrap;
491→}
492→.ks-seg-btn {
493→  padding: 6px 10px;
494→  border-radius: 6px;
495→  border: 1px solid rgba(0, 120, 212, 0.35);
496→  background: transparent;
497→  color: #0078d4;
498→  font-size: 12px;
499→  font-weight: 600;
500→  cursor: pointer;
501→  transition: background 0.18s ease, color 0.18s ease, border-color 0.18s ease;
502→}
503→.ks-seg-btn:hover {
504→  background: rgba(0, 120, 212, 0.08);
505→}
506→.ks-seg-btn.active {
507→  background: #0078d4;
508→  color: #fff;
509→  border-color: #0078d4;
510→}
511→html.theme-dark .ks-seg-btn {
512→  border-color: rgba(76, 194, 255, 0.4);
513→  color: #4cc2ff;
514→}
515→html.theme-dark .ks-seg-btn.active {
516→  background: #4cc2ff;
517→  color: #1a1a1a;
518→  border-color: #4cc2ff;
519→}
520→.ks-color-controls {
521→  display: flex;
522→  flex-direction: column;
523→  gap: 10px;
524→}
525→.ks-color-input {
526→  width: 56px;
527→  height: 32px;
528→  padding: 2px;
529→  border-radius: 6px;
530→  border: 1px solid rgba(0, 120, 212, 0.35);
531→  background: transparent;
532→  cursor: pointer;
533→}
534→html.theme-dark .ks-color-input {
535→  border-color: rgba(76, 194, 255, 0.4);
536→}
537→</style>