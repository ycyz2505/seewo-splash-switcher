<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  api,
  type Config,
  type Status,
  type ThemeInfo,
  type UpdateInfo,
} from "./api";
import ThemesView from "./views/ThemesView.vue";
import SettingsView from "./views/SettingsView.vue";
import StatusView from "./views/StatusView.vue";
import LogsView from "./views/LogsView.vue";

type Tab = "themes" | "settings" | "status" | "logs";

const tab = ref<Tab>("themes");
const config = ref<Config | null>(null);
const status = ref<Status | null>(null);
const themes = ref<ThemeInfo[]>([]);
const update = ref<UpdateInfo | null>(null);
const loading = ref(true);
const busy = ref(false);
const toast = ref("");
const toastError = ref(false);
let toastTimer: number | undefined;

const activeThemeName = computed(
  () => themes.value.find((t) => t.active)?.name ?? config.value?.activeTheme ?? "—",
);

function showToast(message: string, error = false) {
  toast.value = message;
  toastError.value = error;
  if (toastTimer) window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.value = "";
  }, 4200);
}

function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

async function refreshAll() {
  try {
    const [c, s, t] = await Promise.all([
      api.getConfig(),
      api.getStatus(),
      api.listThemes(),
    ]);
    config.value = c;
    status.value = s;
    themes.value = t;
  } catch (e) {
    showToast("加载失败：" + errText(e), true);
  } finally {
    loading.value = false;
  }
}

async function selectTheme(id: string) {
  busy.value = true;
  try {
    await api.setActiveTheme(id);
    await refreshAll();
    showToast("已切换到该主题");
  } catch (e) {
    showToast("切换失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function importTheme() {
  busy.value = true;
  try {
    const id = await api.importTheme();
    if (id) {
      await refreshAll();
      showToast("已导入主题：" + id);
    }
  } catch (e) {
    showToast("导入失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function deleteTheme(id: string) {
  if (!window.confirm(`确定删除主题「${id}」吗？`)) return;
  busy.value = true;
  try {
    await api.deleteTheme(id);
    await refreshAll();
    showToast("已删除主题");
  } catch (e) {
    showToast("删除失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function saveConfig(next: Config) {
  busy.value = true;
  try {
    await api.saveConfig(next);
    config.value = next;
    await refreshAll();
    showToast("设置已保存");
  } catch (e) {
    showToast("保存失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function runApply(restore = false) {
  busy.value = true;
  try {
    const report = await api.runApply(restore);
    await refreshAll();
    if (report.errors.length) {
      showToast("应用完成，但有警告：\n" + report.errors.join("\n"), true);
    } else {
      showToast(report.message || (restore ? "已恢复原状" : "已应用当前配置"));
    }
  } catch (e) {
    showToast("应用失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function testSplash() {
  busy.value = true;
  try {
    await api.testSplash();
    showToast("已启动测试启动页");
  } catch (e) {
    showToast("测试失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function checkUpdates() {
  busy.value = true;
  try {
    update.value = await api.checkUpdates();
    showToast(update.value ? `发现新版本 v${update.value.version}` : "已是最新版本");
  } catch (e) {
    showToast("检查更新失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function installUpdate() {
  busy.value = true;
  try {
    await api.installUpdate();
    showToast("更新已开始，程序会自动重启");
  } catch (e) {
    showToast("安装更新失败：" + errText(e), true);
  } finally {
    busy.value = false;
  }
}

async function openPath(path: string) {
  try {
    await api.openPath(path);
  } catch (e) {
    showToast("打开失败：" + errText(e), true);
  }
}

onMounted(refreshAll);
</script>

<template>
  <div class="app">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark">希</div>
        <div>
          <div class="brand-title">希沃启动页切换器</div>
          <div class="brand-sub">HTML Splash · Tauri</div>
        </div>
      </div>

      <nav class="nav">
        <button :class="{ active: tab === 'themes' }" @click="tab = 'themes'">
          <span class="nav-icon">◈</span> 启动页主题
        </button>
        <button :class="{ active: tab === 'settings' }" @click="tab = 'settings'">
          <span class="nav-icon">⚙</span> 设置
        </button>
        <button :class="{ active: tab === 'status' }" @click="tab = 'status'">
          <span class="nav-icon">◎</span> 状态与接管
        </button>
        <button :class="{ active: tab === 'logs' }" @click="tab = 'logs'">
          <span class="nav-icon">≡</span> 日志
        </button>
      </nav>

      <div class="sidebar-foot">
        <div>当前主题：{{ activeThemeName }}</div>
        <div v-if="status">v{{ status.appVersion }}</div>
        <div v-if="status">服务端口：{{ status.serverPort }}</div>
      </div>
    </aside>

    <main class="main">
      <div v-if="update" class="banner">
        <div>
          发现新版本 <b>v{{ update.version }}</b>
          <span class="muted"> · {{ update.notes || "无更新说明" }}</span>
        </div>
        <button class="btn primary" :disabled="busy" @click="installUpdate">
          立即更新
        </button>
      </div>

      <div v-if="loading" class="empty">正在加载…</div>

      <template v-else>
        <ThemesView
          v-if="tab === 'themes'"
          :themes="themes"
          :active-theme="config?.activeTheme ?? ''"
          :busy="busy"
          @select="selectTheme"
          @import="importTheme"
          @delete="deleteTheme"
          @open="openPath"
          @refresh="refreshAll"
        />
        <SettingsView
          v-else-if="tab === 'settings' && config"
          :config="config"
          :busy="busy"
          @save="saveConfig"
          @open="openPath"
        />
        <StatusView
          v-else-if="tab === 'status' && status"
          :status="status"
          :update="update"
          :busy="busy"
          @apply="runApply(false)"
          @restore="runApply(true)"
          @test="testSplash"
          @check-update="checkUpdates"
          @install-update="installUpdate"
          @open="openPath"
        />
        <LogsView v-else-if="tab === 'logs'" @open="openPath" />
      </template>
    </main>

    <div v-if="toast" class="toast" :class="{ error: toastError }">{{ toast }}</div>
  </div>
</template>
