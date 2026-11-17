<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { api } from "../api";

const files = ["app.log", "apply.log", "splash.log", "updater.log", "diagnose.log"];
const selected = ref(files[0]);
const content = ref("");
const loading = ref(false);
let timer: number | undefined;

async function load() {
  loading.value = true;
  try {
    content.value = await api.readLog(selected.value);
  } catch (error) {
    content.value = "读取失败：" + String(error);
  } finally {
    loading.value = false;
  }
}

watch(selected, load);

onMounted(() => {
  load();
  timer = window.setInterval(load, 3000);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});
</script>

<template>
  <div class="page-head">
    <div>
      <div class="page-title">日志</div>
      <div class="page-desc">每 3 秒自动刷新。日志写在 D: 的 <span class="mono">logs/</span> 下。</div>
    </div>
    <div class="actions">
      <select v-model="selected" class="select">
        <option v-for="file in files" :key="file" :value="file">{{ file }}</option>
      </select>
      <button class="btn" :disabled="loading" @click="load">刷新</button>
      <button class="btn" @click="api.openLogsDir()">打开日志目录</button>
    </div>
  </div>

  <pre class="logs">{{ content || "（空）" }}</pre>
</template>
