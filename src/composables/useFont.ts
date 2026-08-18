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
const STYLE_ID = "toolsplus-font-style";

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

function ensureStyleEl(): HTMLStyleElement {
  let el = document.getElementById(STYLE_ID) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = STYLE_ID;
    document.head.appendChild(el);
  }
  return el;
}

const ICON_FONT_STACK =
  "'Segoe Fluent Icons', 'Segoe MDL2 Assets', 'WinUIOnWebIcons', 'Segoe UI Symbol'";

function applyFontFamily(f: FontFamily) {
  const stack = fontStackMap[f] || fontStackMap.system;
  const html = document.documentElement;
  const body = document.body;
  html.style.setProperty("--app-font-family", stack);
  html.style.setProperty("--ContentControlThemeFontFamily", stack);
  if (body) {
    body.style.setProperty("--app-font-family", stack);
    body.style.setProperty("--ContentControlThemeFontFamily", stack);
  }
  const css = `
html, body, #app, .app-root, .app-shell,
.win-text-block, .win-text, .win-button, .win-button-content,
.win-nav-item-content, .win-settings-card, .win-expander-content,
.win-combo-box, .win-text-box, .win-radio-button, .win-check-box,
.win-content-dialog-title, .win-content-dialog-body,
.win-menu-flyout-item-text, .win-list-view-item-content,
.win-breadcrumb-bar-item, .win-pivot-header, .win-info-bar-content,
.win-grid-view-item-content, .win-hyperlink-button {
  font-family: ${stack} !important;
}
.win-expander-header-icon, .win-settings-card-icon,
.win-expander-header-icon *, .win-settings-card-icon *,
.symbol-icon, .appbar-button-chevron,
.win-symbol-icon, .win-font-icon, .win-icon,
.win-pivot-header-icon, .win-info-bar-icon,
.win-menu-flyout-icon, .win-list-view-item-icon,
.win-breadcrumb-bar-icon, .win-command-bar-icon {
  font-family: ${ICON_FONT_STACK} !important;
}
`;
  ensureStyleEl().textContent = css;
}

function applyAntialias(m: AntialiasMode) {
  const html = document.documentElement;
  const body = document.body;
  const val = aaValueMap[m] || "auto";
  const renderMode = m === "none" ? "optimizeSpeed" : "optimizeLegibility";
  html.style.setProperty("-webkit-font-smoothing", val);
  html.style.setProperty("--app-font-smoothing", val);
  html.style.textRendering = renderMode;
  if (body) {
    body.style.setProperty("-webkit-font-smoothing", val);
    body.style.setProperty("--app-font-smoothing", val);
    body.style.textRendering = renderMode;
  }
  const css = `
html, body, #app, .app-root, .app-shell,
.win-text-block, .win-text, .win-button, .win-button-content,
.win-nav-item-content, .win-settings-card, .win-expander-content,
.win-combo-box, .win-text-box, .win-radio-button, .win-check-box,
.win-content-dialog-title, .win-content-dialog-body,
.win-menu-flyout-item-text, .win-list-view-item-content,
.win-breadcrumb-bar-item, .win-pivot-header, .win-info-bar-content,
.win-grid-view-item-content, .win-hyperlink-button {
  -webkit-font-smoothing: ${val} !important;
  text-rendering: ${renderMode} !important;
}
`;
  let el = document.getElementById(STYLE_ID + "-aa") as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = STYLE_ID + "-aa";
    document.head.appendChild(el);
  }
  el.textContent = css;
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
