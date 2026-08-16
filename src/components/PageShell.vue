<script setup lang="ts">
import { inject, computed } from "vue";
import WinScrollViewer from "@winui/components/WinScrollViewer.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const props = defineProps<{
  titleKey: string;
  subtitleKey?: string;
}>();

const i18n = inject<I18n>(i18nKey)!;
const title = computed(() => i18n.t(props.titleKey));
const subtitle = computed(() => (props.subtitleKey ? i18n.t(props.subtitleKey) : ""));
</script>

<template>
  <WinScrollViewer class="page-scroll">
    <div class="page-root">
      <header class="page-header">
        <h1 class="page-title">{{ title }}</h1>
        <WinTextBlock v-if="subtitle" :Text="subtitle" Style="font-size:13px;opacity:.7" Foreground="secondary" />
      </header>
      <div class="page-body">
        <slot />
      </div>
    </div>
  </WinScrollViewer>
</template>

<style scoped>
.page-scroll {
  width: 100%;
  height: 100%;
}
.page-root {
  padding: 24px 36px 48px;
  max-width: 1064px;
  margin: 0 auto;
  box-sizing: border-box;
}
.page-header {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 24px;
}
.page-title {
  font-size: 28px;
  font-weight: 600;
  line-height: 36px;
  margin: 0;
  color: var(--TextFillColorPrimaryBrush, var(--text-primary, inherit));
}
.page-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
</style>
