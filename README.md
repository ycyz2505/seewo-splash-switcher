# 希沃启动页切换器

用 HTML 替换希沃白板（EasiNote 5）的启动画面。程序本身是一个 Tauri 2 + Vue 3 的便携应用，所有自定义文件放在 D:，不修改希沃的 DLL/EXE；通过快捷方式接管和 Win32 窗口控制，实现“先显示 HTML，点击后露出希沃主窗口”。

- 支持多个 HTML 主题、导入 ZIP、预览、切换
- 780×522 居中 WebView2 启动页（与原版启动画面同尺寸），支持 CSS/JS/Canvas/WebGL/视频
- 后台启动希沃，自动隐藏原版 `Splash Image` 窗口
- 点击“进入”后把希沃主窗口切到前台
- 开机 `--boot` 自动重新接管 C:，并检查/安装更新
- 自动更新从 GitHub Releases 读取 `latest.json`，内置多个加速源
- 所有持久化文件在 D:，C: 重启还原不影响

## 快速开始

1. 从 [Releases](https://github.com/ycyz2505/seewo-splash-switcher/releases) 下载 `SeewoHtmlSplash-v*-portable.zip`。
2. 解压到 `D:\SeewoHtmlSplash`（路径可以换，但建议放 D:）。
3. 双击 `启动希沃启动页切换器.bat`。
4. 在“启动页主题”里选择主题；在“状态与接管”里点“一键修复”，按 UAC 提示允许。
5. 之后正常点桌面/开始菜单的希沃图标，就会先出现 HTML 启动页。

如果希沃已经在运行，默认会跳过启动页；可以在设置里关闭“希沃已运行时跳过”。

## 目录结构

```
D:\SeewoHtmlSplash\
  app\
    seewo-splash-switcher.exe   主程序（GUI / --splash / --boot）
    seewo-apply.exe             提权助手，负责改快捷方式/计划任务
  tools\
    seewo-updater.exe           更新替换器
    seewo-launcher-shim.exe     可选的 swenlauncher 代理
    apply-shortcuts.ps1 等      运行时写入的 PowerShell 脚本
  data\
    config.json                 当前配置
    themes\<id>\                 HTML 主题
    apply-report.json           上次接管结果
    webview2\                   WebView2 用户数据
  backup\
    shortcuts\                  原快捷方式备份
  updates\
    download\                   更新包下载缓存
    staging\                    解压后的新版本
  logs\
    app.log / apply.log / splash.log / updater.log
```

## C: 重启还原

学校电脑常见 C: 重启还原、D: 不还原。本程序的设计是：

- **持久化全部放 D:**：exe、主题、配置、日志、WebView2 缓存、更新缓存。
- **C: 只做可重放的三件事**：
  1. 把桌面/开始菜单的希沃快捷方式指向本程序；
  2. 可选：把 `swenlauncher.exe` 换成代理，覆盖双击 `.enbx` 课件；
  3. 创建登录计划任务 `SeewoHtmlSplash\Boot`，以最高权限运行 `--boot --silent`。
- C: 还原后，计划任务/快捷方式可能被清除。下次从 D: 打开本程序，点“一键修复”即可重新接管；如果计划任务还在，开机会自动执行 `--boot`。

`--boot` 会先重新应用接管，再检查更新；如果开启“开机自动安装”，发现新版本会下载、替换 `app`、健康检查、失败回滚，然后自动重启。

## 主题格式

主题放在 `data\themes\<id>\`，至少包含：

```
manifest.json
index.html
```

`manifest.json`：

```json
{
  "schemaVersion": 1,
  "id": "genshin",
  "name": "原神风格",
  "description": "仿原神启动页",
  "version": "1.0.0",
  "entry": "index.html",
  "preview": "preview.png",
  "options": {
    "audio": { "type": "boolean", "default": true, "label": "播放音乐" }
  }
}
```

`index.html` 里引入宿主提供的 SDK：

```html
<script src="/_sdk.js"></script>
<script>
  document.body.addEventListener("click", () => Seewo.enter());
  Seewo.onStatus((s) => {
    // s.state / s.seewoRunning / s.mainReady / s.entered / s.elapsedMs / s.error
  });
</script>
```

SDK：

- `Seewo.enter()`：请求进入；宿主会关闭覆盖层并把希沃主窗口切到前台。
- `Seewo.status()`：返回当前状态 JSON。
- `Seewo.onStatus(cb, intervalMs)`：轮询状态，返回停止函数。
- `Seewo.config()`：返回当前配置、主题 id 和主题 options。

主题包可以是 ZIP：根目录或单个顶层文件夹里放 `manifest.json` 即可。导入时会校验路径，拒绝 `..` 和绝对路径。

## 自动更新

发布时 GitHub Actions 会生成：

```
SeewoHtmlSplash-v0.2.0-portable.zip
SeewoHtmlSplash-v0.2.0-portable.zip.sig   （配置签名密钥后）
latest.json
latest.json.sig                          （配置签名密钥后）
mirrors.json
sha256sums.txt
```

`latest.json` 里的 `asset.url` 是 GitHub 直链，`asset.mirrors` 是加速源。客户端会：

1. 并行请求直链和内置加速源的 `latest.json`，带超时；
2. 用 minisign 公钥验签（配置了公钥时），选择版本最高的清单；
3. 按直链、清单 mirrors、内置模板、自定义源依次下载；
4. 校验文件大小、SHA-256、zip 的 minisign 签名；
5. 解压到 `updates\staging\<version>`，交给 `tools\seewo-updater.exe`；
6. 更新器等主程序退出，替换 `app`，跑 `--health-check`，成功则重启，失败则回滚到 `app.old.*`。

内置加速源（可在设置里追加自定义模板，`{url}` 会被替换）：

- `https://ghproxy.com/{url}`
- `https://ghfast.top/{url}`
- `https://gh-proxy.com/{url}`
- `https://hub.gitmirror.com/{url}`

第三方加速源能看到请求 URL 和 IP，但内容由签名保证；不要通过镜像发送凭据。

### 下载代理

设置 → 自动更新 → 下载代理：

- **系统代理（默认）**：读取 `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings` 的 `ProxyEnable` / `ProxyServer`。
- **直连**：不使用任何代理（包括环境变量里的 `HTTP_PROXY` / `HTTPS_PROXY`）。
- **手动代理**：填写 `http://127.0.0.1:7890`、`socks5://127.0.0.1:7891` 等。

不支持 PAC 自动配置脚本；如果系统使用 PAC，请改用手动代理或直连。改完设置后可以点“检查更新”验证代理是否可用。

命令行也可以只检查更新（结果写入 `logs\app.log`，不修改 C:）：

```bash
seewo-splash-switcher.exe --check-update
```

### 本地 ZIP 更新（加速源下载失败时）

如果当前网络下 GitHub/加速源的大文件下载失败，可以手动下载 Release 里的 `SeewoHtmlSplash-vX-portable.zip`，然后：

- GUI：打开“状态与接管”，点“从本地 ZIP 更新”，选择下载好的 zip。
- 命令行：

  ```bash
  seewo-splash-switcher.exe --install-zip D:\SeewoHtmlSplash\download\SeewoHtmlSplash-vX-portable.zip
  ```

程序会把 zip 解压到 `updates\staging\local-*`，调用 `tools\seewo-updater.exe` 替换 `app`、做健康检查、失败回滚，并顺带更新默认主题；`data\config.json` 和用户自己导入的主题不会被覆盖。

## 构建与发布

不需要本地构建，全部在 GitHub Actions 的 `windows-latest` 上完成。

1. 把仓库推到 GitHub。
2. 生成签名密钥（一次性）：

   ```bash
   npx tauri signer generate -w .secrets/minisign.key
   ```

   把 `.key.pub` 的内容替换 `src-tauri/keys/minisign.pub`（第一行注释 + 第二行 base64 公钥）。
   在仓库 Secrets 里添加：

   - `TAURI_SIGNING_PRIVATE_KEY`：`.secrets/minisign.key` 的内容
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：生成时设置的密码

3. 推送 tag：

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

   或在 Actions 页面手动运行 `release`，输入版本号。

Workflow 会：设置版本号 → `npm install` → 构建前端 → `tauri build --no-bundle` → 构建 helper → 打包 portable zip → 生成 `latest.json`/`mirrors.json` → 签名（如果 Secret 存在）→ 创建/上传/发布 Release → 上传 artifact。

## 限制与风险

- 本程序不改希沃的 DLL/EXE，不会触发 `ChecksumMd5.txt` 的完整性校验。
- 快捷方式、`swenlauncher.exe` 代理、计划任务在 C:，C: 还原后需要重新应用。
- 如果学校启用 AppLocker/软件限制策略，D: 上的自制 exe 可能被拦截；需要找管理员加白名单。
- 自定义 exe 未签名时，SmartScreen 可能提示；可以点“更多信息 → 仍要运行”，或自行签名。
- 希沃更新后窗口类名/启动逻辑可能变化；程序用进程/窗口枚举和 `Splash Image` 多条件匹配，必要时更新代码。
- `swenlauncher.exe` 代理默认关闭；开启后会替换希沃的签名 launcher，风险更高，但能覆盖双击 `.enbx` 课件的入口。
- 仅用于个人/学校电脑的本地定制，不要分发修改后的希沃文件。

## 技术栈

- Tauri 2（Rust）+ WebView2
- Vue 3 + Vite + TypeScript
- Axum 本地 HTTP 服务（127.0.0.1 随机端口）
- Win32 `FindWindow`/`ShowWindow`/`EnumWindows`/`SetForegroundWindow`
- minisign 验签（`minisign-verify`）
- GitHub Actions 自动构建与发布

## License

MIT
