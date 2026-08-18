import { ref, watch } from "vue";

export type FontFamily =
  | "system"
  | "segoe"
  | "yahei"
  | "simsun"
  | "kaiti"
  | "consolas";

export type AntialiasMode = "auto" | "antialiased" | "subpixel-antialiased" | "none";

const FONT_KEY = "toolsplus-font";
const AA_KEY = "toolsplus-antialias";

const fontStackMap: Record<FontFamily, string> = {
  system: "system-ui, 'Segoe UI', 'Microsoft YaHei UI', sans-serif",
  segoe: "'Segoe UI', 'Microsoft YaHei UI', sans-serif",
  yahei: "'Microsoft YaHei UI', 'Microsoft YaHei', sans-serif",
  simsun: "'SimSun', 'NSimSun', serif",
  kaiti: "'KaiTi', 'STKaiti', cursive",
  consolas: "'Consolas', 'Cascadia Code', 'JetBrains Mono', monospace",
};

const aaValueMap: Record<AntialiasMode, string> = {
  auto: "auto",
  antialiased: "antialiased",
  "subpixel-antialiased": "subpixel-antialiased",
  none: "none",
};

function loadFont(): FontFamily {
  const v = localStorage.getItem(FONT_KEY) as FontFamily | null;
  if (v && v in fontStackMap) return v;
  return "system";
}

function loadAA(): AntialiasMode {
  const v = localStorage.getItem(AA_KEY) as AntialiasMode | null;
  if (v && v in aaValueMap) return v;
  return "auto";
}

export const fontFamily = ref<FontFamily>(loadFont());
export const antialias = ref<AntialiasMode>(loadAA());

export function applyFontFamily(f: FontFamily) {
  const stack = fontStackMap[f] || fontStackMap.system;
  document.documentElement.style.setProperty("--app-font-family", stack);
  document.documentElement.style.fontFamily = stack;
  document.body.style.fontFamily = stack;
  const app = document.getElementById("app");
  if (app) (app.style as any).fontFamily = stack;
  document.querySelectorAll<HTMLElement>(".win-expander, .win-settings-card").forEach((el) => {
    el.style.fontFamily = stack;
  });
}

export function applyAntialias(m: AntialiasMode) {
  const html = document.documentElement;
  const body = document.body;
  const val = aaValueMap[m] || "auto";
  html.style.setProperty("-webkit-font-smoothing", val);
  body.style.setProperty("-webkit-font-smoothing", val);
  const renderMode = m === "none" ? "optimizeSpeed" : "optimizeLegibility";
  html.style.textRendering = renderMode;
  body.style.textRendering = renderMode;
}

export function setFontFamily(f: FontFamily) {
  fontFamily.value = f;
  localStorage.setItem(FONT_KEY, f);
  applyFontFamily(f);
}

export function setAntialias(m: AntialiasMode) {
  antialias.value = m;
  localStorage.setItem(AA_KEY, m);
  applyAntialias(m);
}

export function initFont() {
  applyFontFamily(fontFamily.value);
  applyAntialias(antialias.value);
}

watch(fontFamily, (v) => applyFontFamily(v));
watch(antialias, (v) => applyAntialias(v));

export function useFont() {
  return { fontFamily, antialias, setFontFamily, setAntialias };
}
