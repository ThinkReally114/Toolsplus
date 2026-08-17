<script setup lang="ts">
import { onMounted, ref } from "vue";

const mounted = ref(false);
const error = ref("");

onMounted(() => {
  mounted.value = true;
  console.log("ToolsPlus mounted successfully!");
});
</script>

<template>
  <div style="
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-family: 'Segoe UI', system-ui, sans-serif;
    color: #333;
    background: #F3F3F3;
    padding: 40px;
  ">
    <h1 style="font-size: 28px; margin-bottom: 16px; color: #0078d4;">
      ToolsPlus
    </h1>
    <p v-if="mounted" style="font-size: 16px;">
      ✅ Vue mounted successfully!
    </p>
    <p v-else style="font-size: 16px; color: #999;">
      ⏳ Loading...
    </p>
    <p v-if="error" style="color: #c00; margin-top: 20px;">
      Error: {{ error }}
    </p>
  </div>
</template>