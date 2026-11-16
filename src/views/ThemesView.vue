<script setup lang="ts">
import { computed } from "vue";
import type { ThemeInfo } from "../api";

const props = defineProps<{
  themes: ThemeInfo[];
  activeTheme: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: "select", id: string): void;
  (e: "import"): void;
  (e: "delete", id: string): void;
  (e: "open", path: string): void;
  (e: "refresh"): void;
}>();

const active = computed(() => props.themes.find((theme) => theme.active));

function hideBrokenImage(event: Event) {
  const image = event.target as HTMLImageElement;
  image.style.display = "none";
}
</script>

<template>
  <div class="page-head">
    <div>
      <div class="page-title">启动页主题</div>
      <div class="page-desc">
        支持单个自包含 HTML，或带资源的 ZIP（根目录 index.html，可选 manifest.json）。当前主题：{{ active?.name ?? "—" }}。
      </div>
    </div>
    <div class="actions">
      <button class="btn" :disabled="busy" @click="emit('refresh')">刷新</button>
      <button class="btn primary" :disabled="busy" @click="emit('import')">导入主题（HTML/ZIP）</button>
      <button class="btn ghost" :disabled="!active" @click="active && emit('open', active.path)">
        打开主题目录
      </button>
    </div>
  </div>

  <div v-if="!themes.length" class="empty">
    还没有主题。点击“导入主题（HTML/ZIP）”，或把主题文件夹放到 <span class="mono">data/themes</span> 下。
  </div>

  <div v-else class="grid themes-grid">
    <article v-for="theme in themes" :key="theme.id" class="card theme-card">
      <div class="theme-preview">
        <img
          v-if="theme.previewUrl"
          :src="theme.previewUrl"
          :alt="theme.name"
          @error="hideBrokenImage"
        />
        <div class="fallback">希</div>
      </div>
      <div class="theme-body">
        <div class="theme-name">
          {{ theme.name }}
          <span v-if="theme.active" class="badge good">使用中</span>
          <span class="badge">{{ theme.version }}</span>
        </div>
        <div class="theme-desc">{{ theme.description || "暂无描述" }}</div>
        <div class="theme-actions">
          <button
            class="btn primary"
            :disabled="theme.active || busy"
            @click="emit('select', theme.id)"
          >
            {{ theme.active ? "当前主题" : "使用" }}
          </button>
          <button
            class="btn danger"
            :disabled="theme.active || busy"
            @click="emit('delete', theme.id)"
          >
            删除
          </button>
        </div>
      </div>
    </article>
  </div>
</template>
