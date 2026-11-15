<script setup lang="ts">
import { ref, watch } from "vue";
import { api, type Config } from "../api";

const props = defineProps<{
  config: Config;
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: "save", config: Config): void;
}>();

function clone(config: Config): Config {
  return JSON.parse(JSON.stringify(config)) as Config;
}

const draft = ref<Config>(clone(props.config));
const customMirrorsText = ref(props.config.update.customMirrors.join("\n"));

watch(
  () => props.config,
  (config) => {
    draft.value = clone(config);
    customMirrorsText.value = config.update.customMirrors.join("\n");
  },
  { deep: true },
);

function save() {
  const next = clone(draft.value);
  next.update.customMirrors = customMirrorsText.value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  emit("save", next);
}
</script>

<template>
  <div class="page-head">
    <div>
      <div class="page-title">设置</div>
      <div class="page-desc">
        所有配置保存在 D: 的 <span class="mono">data/config.json</span>，不会写 C: 的注册表。
      </div>
    </div>
    <div class="actions">
      <button class="btn" :disabled="busy" @click="api.openDataDir()">打开数据目录</button>
      <button class="btn" :disabled="busy" @click="api.openLogsDir()">打开日志目录</button>
      <button class="btn primary" :disabled="busy" @click="save">保存设置</button>
    </div>
  </div>

  <div class="grid" style="grid-template-columns: repeat(auto-fit, minmax(340px, 1fr))">
    <section class="card">
      <div class="section-title">启动页</div>

      <div class="row">
        <div>
          <div class="row-label">启用 HTML 启动页</div>
          <div class="row-hint">关闭后直接启动希沃，不显示自定义页面。</div>
        </div>
        <label class="switch"><input v-model="draft.enabled" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">启动希沃</div>
          <div class="row-hint">显示 HTML 的同时在后台启动希沃白板。</div>
        </div>
        <label class="switch"><input v-model="draft.splash.launchSeewo" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">隐藏原版启动画面</div>
          <div class="row-hint">检测并隐藏希沃的 Splash Image 窗口。</div>
        </div>
        <label class="switch"><input v-model="draft.splash.hideOriginalSplash" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">希沃已运行时跳过</div>
          <div class="row-hint">避免每次点快捷方式都弹出启动页。</div>
        </div>
        <label class="switch"><input v-model="draft.splash.skipIfRunning" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">透传启动参数</div>
          <div class="row-hint">双击 .enbx 课件时把文件路径传给希沃。</div>
        </div>
        <label class="switch"><input v-model="draft.splash.forwardArgs" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">等待超时（秒）</div>
          <div class="row-hint">超过这个时间仍未点击，自动进入希沃。</div>
        </div>
        <input v-model.number="draft.splash.timeoutSeconds" class="input num" type="number" min="10" max="600" />
      </div>
    </section>

    <section class="card">
      <div class="section-title">接管 C:</div>

      <div class="row">
        <div>
          <div class="row-label">接管快捷方式</div>
          <div class="row-hint">把桌面/开始菜单的希沃快捷方式指向本程序。C: 还原后需重新应用。</div>
        </div>
        <label class="switch"><input v-model="draft.takeover.shortcuts" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">替换 swenlauncher.exe</div>
          <div class="row-hint">
            覆盖双击 .enbx 课件的入口；会备份原文件。默认关闭，风险更高。
          </div>
        </div>
        <label class="switch"><input v-model="draft.takeover.launcherShim" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">开机自动应用</div>
          <div class="row-hint">创建登录计划任务，以最高权限运行 --boot。C: 还原后可能被清除。</div>
        </div>
        <label class="switch"><input v-model="draft.takeover.autostart" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">C: 还原说明</div>
          <div class="row-hint">
            程序本体、主题、配置都在 D:。每次开机从 D: 打开本程序，或在状态页点“一键修复”重新接管 C:。
          </div>
        </div>
      </div>
    </section>

    <section class="card">
      <div class="section-title">自动更新</div>

      <div class="row">
        <div>
          <div class="row-label">启用更新检查</div>
          <div class="row-hint">从 GitHub Releases 读取 latest.json，支持多个加速源。</div>
        </div>
        <label class="switch"><input v-model="draft.update.enabled" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">开机自动安装</div>
          <div class="row-hint">--boot 时发现新版本就下载并替换 app，然后自动重启。</div>
        </div>
        <label class="switch"><input v-model="draft.update.autoInstall" type="checkbox" /><span></span></label>
      </div>

      <div class="row">
        <div>
          <div class="row-label">检查间隔（小时）</div>
          <div class="row-hint">GUI 和托盘按这个间隔检查更新。</div>
        </div>
        <input v-model.number="draft.update.checkIntervalHours" class="input num" type="number" min="1" max="168" />
      </div>

      <div class="row">
        <div>
          <div class="row-label">下载代理</div>
          <div class="row-hint">自动更新检查和下载使用的代理。改完可点“检查更新”验证。</div>
        </div>
        <select v-model="draft.update.proxyMode" class="select">
          <option value="system">系统代理（推荐）</option>
          <option value="direct">直连（不使用代理）</option>
          <option value="manual">手动代理</option>
        </select>
      </div>

      <div v-if="draft.update.proxyMode === 'manual'" class="row">
        <div>
          <div class="row-label">手动代理地址</div>
          <div class="row-hint">例如 http://127.0.0.1:7890 或 socks5://127.0.0.1:7891。</div>
        </div>
        <input
          v-model="draft.update.proxyUrl"
          class="input"
          style="width: 260px"
          placeholder="http://127.0.0.1:7890"
        />
      </div>

      <div class="row" style="display: block">
        <div class="row-label">自定义加速源</div>
        <div class="row-hint" style="margin-bottom: 8px">
          每行一个，支持 <span class="mono">{url}</span> 占位符，例如
          <span class="mono">https://ghfast.top/{url}</span>。留空则只用内置源。
        </div>
        <textarea v-model="customMirrorsText" class="textarea" spellcheck="false"></textarea>
      </div>
    </section>
  </div>
</template>
