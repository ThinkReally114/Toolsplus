import { ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const PATH_KEY = "toolsplus-home-bg";
const OPACITY_KEY = "toolsplus-home-bg-opacity";

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const imageDataUrl: Ref<string | null> = ref(null);
const imagePath: Ref<string | null> = ref(null);
const opacity: Ref<number> = ref(getOpacity());

export function getHomeBackgroundPath(): string | null {
  return localStorage.getItem(PATH_KEY) || null;
}

export function getOpacity(): number {
  const v = Number(localStorage.getItem(OPACITY_KEY));
  if (isNaN(v)) return 100;
  return Math.min(100, Math.max(10, v));
}

export function homeBackgroundUrl(): Ref<string | null> {
  return imageDataUrl;
}

export function homeBackgroundOpacity(): Ref<number> {
  return opacity;
}

async function loadIntoRef(path: string) {
  if (!hasTauri) {
    imageDataUrl.value = path;
    return;
  }
  try {
    const url = await invoke<string>("read_image_as_data_url", { path });
    imageDataUrl.value = url;
  } catch (e) {
    console.error("load home background failed", e);
    imageDataUrl.value = null;
  }
}

export async function initHomeBackground() {
  const p = getHomeBackgroundPath();
  imagePath.value = p;
  if (p) {
    await loadIntoRef(p);
  } else {
    imageDataUrl.value = null;
  }
}

export async function setHomeBackground(path: string) {
  localStorage.setItem(PATH_KEY, path);
  imagePath.value = path;
  await loadIntoRef(path);
}

export function clearHomeBackground() {
  localStorage.removeItem(PATH_KEY);
  imagePath.value = null;
  imageDataUrl.value = null;
}

export function setHomeBackgroundOpacity(v: number) {
  const clamped = Math.min(100, Math.max(10, v));
  localStorage.setItem(OPACITY_KEY, String(clamped));
  opacity.value = clamped;
}
