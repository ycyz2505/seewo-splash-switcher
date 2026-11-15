<script setup lang="ts">
import { api, formatBytes, type Status, type UpdateInfo } from "../api";

defineProps<{
  status: Status;
  update: UpdateInfo | null;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: "apply"): void;
  (e: "restore"): void;
  (e: "test"): void;
  (e: "check-update"): void;
  (e: "install-update"): void;
}>();
</script>

<template>
  <div class="page-head">
    <div>
      <div class="page-title">状态与接管</div>
      <div class="page-desc">
        查看希沃路径、C: 接管状态和更新状态。C: 还原后，点“一键修复”重新应用即可。
      </div>
    </div>
    <div class="actions">
      <button class="btn" :disabled="busy" @click="emit('test')">测试启动页</button>
      <button class="btn" :disabled="busy" @click="emit('check-update')">检查更新</button>
      <button class="btn primary" :disabled="busy" @click="emit('apply')">一键修复</button>
      <button class="btn danger" :disabled="busy" @click="emit('restore')">恢复原状</button>
    </div>
  </div>

  <div class="status-grid">
    <div class="stat">
      <div class="stat-label">希沃路径</div>
      <div class="stat-value mono">{{ status.seewoPath || "未检测到" }}</div>
    </div>
    <div class="stat">
      <div class="stat-label">希沃版本</div>
      <div class="stat-value">{{ status.seewoVersion || "—" }}</div>
    </div>
    <div class="stat">
      <div class="stat-label">当前主题</div>
      <div class="stat-value">{{ status.activeTheme }}</div>
    </div>
    <div class="stat">
      <div class="stat-label">本地服务端口</div>
      <div class="stat-value mono">{{ status.serverPort }}</div>
    </div>
    <div class="stat">
      <div class="stat-label">管理员权限</div>
      <div class="stat-value" :class="status.isAdmin ? 'good' : 'warn'">
        {{ status.isAdmin ? "是" : "否（应用时会请求 UAC）" }}
      </div>
    </div>
    <div class="stat">
      <div class="stat-label">WebView2</div>
      <div class="stat-value mono">{{ status.webview2Version }}</div>
    </div>
    <div class="stat">
      <div class="stat-label">快捷方式接管</div>
      <div class="stat-value" :class="status.shortcutsOk ? 'good' : 'bad'">
        {{ status.shortcutsOk ? "已接管" : "未接管" }}
      </div>
    </div>
    <div class="stat">
      <div class="stat-label">launcher shim</div>
      <div class="stat-value" :class="status.launcherShim ? 'good' : 'warn'">
        {{ status.launcherShim ? "已替换" : "未替换" }}
      </div>
    </div>
    <div class="stat">
      <div class="stat-label">开机自启</div>
      <div class="stat-value" :class="status.autostartOk ? 'good' : 'bad'">
        {{ status.autostartOk ? "已创建" : "未创建" }}
      </div>
    </div>
    <div class="stat">
      <div class="stat-label">上次应用</div>
      <div class="stat-value">{{ status.lastApply || "—" }}</div>
    </div>
  </div>

  <div v-if="update" class="card" style="margin-top: 18px">
    <div class="section-title">可用更新</div>
    <div class="flex" style="justify-content: space-between">
      <div>
        <div class="theme-name">
          v{{ update.version }}
          <span class="badge">{{ formatBytes(update.size) }}</span>
        </div>
        <div class="row-hint" style="margin-top: 6px">{{ update.notes || "无更新说明" }}</div>
      </div>
      <button class="btn primary" :disabled="busy" @click="emit('install-update')">安装更新</button>
    </div>
  </div>

  <div class="card" style="margin-top: 18px">
    <div class="section-title">说明</div>
    <div class="row-hint" style="line-height: 1.9">
      1. 程序本体、主题、配置、更新缓存都在 D:，C: 重启还原不会影响它们。<br />
      2. 快捷方式、swenlauncher shim、登录计划任务在 C:，还原后需要重新应用。<br />
      3. “一键修复”会请求一次管理员权限；计划任务创建成功后，下次开机会自动执行
      <span class="mono">--boot</span>。<br />
      4. 如果学校策略禁止运行 D: 上的 exe，应用会被拦截，需要找管理员加白名单。
    </div>
    <div class="flex" style="margin-top: 12px">
      <button class="btn" @click="api.openDataDir()">打开数据目录</button>
      <button class="btn" @click="api.openLogsDir()">打开日志目录</button>
    </div>
  </div>
</template>
