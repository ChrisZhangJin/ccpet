# ccpet — Tauri v2 Desktop Pet

> 一个常驻桌角、透明置顶、点击穿透的桌面宠物。HTTP `POST /bark` 触发「摇尾 + 吠叫」反应，配套 Claude Code `Stop` hook 在编码任务结束时自动叫一声。

技术栈：**Tauri v2** · Vanilla JS + Vite · Rust (`tiny_http`) · Windows + macOS

---

## ✨ 能力一览

| 能力 | 说明 |
|---|---|
| 🪟 透明置顶 | 桌面角落 300×300 窗口，无边框 + 阴影 + 不进任务栏 |
| 🖱️ 点击穿透 | 鼠标穿过宠物落到下方应用，不挡操作 |
| 🎬 闲置动画 | 持续轻微上下浮动（idle-bob） |
| 🐾 反应动画 | `POST /bark` → 摇尾 + 吠叫脉冲 + 「Woof!」气泡 + 音频，2s 回 idle |
| 🎯 拖动 | **Ctrl + 鼠标拖动**（Windows）/ **⌘ + 拖动**（macOS），不出现标题栏 |
| 🌐 LAN 可达 | HTTP 服务绑定 `0.0.0.0:4242`，LAN 内任意设备可触发 |
| 🖼️ 自动图标 | 替换 `pet.png` → `npm run tauri:build` 自动生成所有尺寸图标 |

---

## 🚀 快速开始（5 分钟上手）

### 前置依赖
- **Node.js** 18+
- **Rust** stable（[rustup.rs](https://rustup.rs)）
- **Windows**：VS Build Tools（"Desktop development with C++" 工作负载）
- **macOS**：`xcode-select --install`

> 国内网络用户首次跑 `cargo` 慢/失败？看 [`docs/cn-network-setup.md`](docs/cn-network-setup.md) 或执行 `bash scripts/setup-cn-network.sh` 一键配镜像。

### 开发模式（热更新）
```bash
npm install
npm run tauri:dev          # 当前平台
npm run tauri:dev:mac      # macOS Apple Silicon 显式指定
```

### 编译发布版
```bash
npm run tauri:build:windows   # Windows x64 → ccpet.exe
npm run tauri:build:mac       # macOS Apple Silicon
npm run tauri:build:mac:intel
npm run tauri:build:mac:universal
```

产物位置：
- Windows：`src-tauri/target/x86_64-pc-windows-msvc/release/ccpet.exe`
- macOS：`src-tauri/target/release/bundle/{macos,dmg}/...`

> **⚠️ MSI/NSIS 安装包打包在国内网络下会卡在 WiX 工具下载**（GitHub CDN 走不通），所以 `bundle.targets` 默认设为 `[]` 只产 exe。要安装包时：`winget install -e --id WiXToolset.WiX` 装本地 WiX，再把 targets 改成 `["msi"]`。

### 手动测试
```bash
curl -X POST http://127.0.0.1:4242/bark
```
应看到宠物播放 ~2s 反应后回 idle。

### Claude Code 集成
把下面合并进 `~/.claude/settings.json`：
```json
{
  "hooks": {
    "Stop": [{
      "hooks": [{
        "type": "command",
        "command": "curl -X POST http://127.0.0.1:4242/bark"
      }]
    }]
  }
}
```

---

## 📂 项目结构
```
ccpet/
├── README.md                   ← 你在这里
├── overview.md                 ← 详细架构 / 验证清单 / 出范围声明
├── package.json                # 脚本: dev / build / tauri:* / gen-icons
├── vite.config.js              # dev :1420, publicDir=src/assets
├── scripts/
│   ├── make-icons.py           # Pillow → icon.ico / icns / PNGs from pet.png
│   └── setup-cn-network.sh     # 一键配 cargo 国内镜像 + 调 timeout
├── src/
│   ├── index.html              # 透明背景入口
│   ├── main.js                 # listen('action') → playReaction() + Ctrl 拖动
│   ├── styles.css              # idle-bob + reaction keyframes
│   └── assets/
│       ├── pet.png             # ← 换成你的宠物（去背 PNG，~200×200）
│       └── bark.mp3            # ← 换成你的叫声
└── src-tauri/
    ├── tauri.conf.json         # 透明 / 置顶 / 穿透 / csp:null
    ├── Cargo.toml              # tauri + tiny_http + windows (Ctrl 轮询)
    ├── capabilities/default.json  # 事件 + 窗口权限
    ├── src/lib.rs              # setup + HTTP server + Ctrl watcher (Windows)
    └── icons/                  # 自动生成（不要手改）
```

---

## ⚠️ 已知踩坑（必读）

### 1. push 一直 403 denied — fine-grained PAT 的「Contents: Read and write」陷阱

**症状**：token 在 GitHub 网页看起来对仓库有 push 权限（你本来就是 owner），API `/repos/.../ccpet` 返回 `permissions.push:True`，但 `git push` 任何分支都返回：
```
remote: Permission to ChrisZhangJin/ccpet.git denied to ChrisZhangJin.
fatal: 403
```

**根因（也是诊断思路，下次遇到直接照搬）**：
- `GET /repos/{owner}/{repo}` 返回的 `permissions.push` 是**用户在该仓库的角色权限**（你是 owner → true），**不是 token 自己的 scope**。
- token 真实权限藏在响应头 **`x-accepted-github-permissions`** 里（curl 加 `-D -` 看）：
  ```
  x-accepted-github-permissions: metadata=read   ← 只有这个？说明 token 没开 Contents 写
  ```
- 更直接的探针：`PUT /repos/{owner}/{repo}/contents/.write-test`，返回 `Resource not accessible by personal access token` 就是 Contents 写缺失。

**修复**：GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens → Edit 该 token：
1. **Repository access** 勾上 `ChrisZhangJin/ccpet`（或 All repositories）
2. **Permissions → Repository permissions → Contents** 改为 **`Read and write`**
3. Save，回到终端重跑 push

**过代理 push 的正确命令**（绕过代理剥离 Authorization 头）：
```bash
cd /d/workspace/ccpet
TOKEN=$(tr -d '[:space:]' < github-token.key)
git -c http.proxy=http://192.168.32.101:18387 \
    -c https.proxy=http://192.168.32.101:18387 \
    -c http.extraheader= \
    push "https://ChrisZhangJin:${TOKEN}@github.com/ChrisZhangJin/ccpet.git" main
```
PowerShell 版：
```powershell
cd D:\workspace\ccpet
$TOKEN = (Get-Content -Raw github-token.key).Trim()
git -c http.proxy=http://192.168.32.101:18387 `
    -c https.proxy=http://192.168.32.101:18387 `
    -c http.extraheader= `
    push "https://ChrisZhangJin:$TOKEN@github.com/ChrisZhangJin/ccpet.git" main
```
> ❌ **不要**用 `git config http.extraheader "Authorization: Bearer <TOKEN>"` 走代理——代理会剥离这个头，GitHub 收到 `invalid credentials`。

### 2. Windows 编译后 Ctrl 拖动不响应

**症状**：按住 Ctrl 鼠标图标不变，宠物拖不动。

**根因**：Windows 上 `setIgnoreCursorEvents(true)` 的窗口**永远拿不到键盘焦点**，所以 `keydown` 事件根本不触发（macOS 上则不受此限制）。

**解决**：已在 `lib.rs` 用 Windows-only `GetAsyncKeyState` 后台轮询 Ctrl 状态，按下/松开时 emit `drag-modifier-{down,up}` 事件给前端，前端再切点击穿透。`#[cfg(target_os = "windows")]` 守卫，macOS 不受影响。

### 3. Windows MSI 安装包打包 `timeout: global`

**症状**：`tauri build` 末尾 `failed to bundle project: timeout: global`，日志显示在下载 `wix314-binaries.zip`。

**根因**：Tauri 从 `github.com/wixtoolset/wix3/releases/...` 下载 WiX 工具，被 302 重定向到 `objects.githubusercontent.com` CDN，国内代理通常只放行 `github.com`，CDN 域被卡 → 超时。

**解决**：默认 `bundle.targets: []`（已在 `tauri.conf.json` 配好），只产 exe。要 MSI 时按上面「快速开始」的 winget 方案装本地 WiX。

---

## 🧪 验证清单（真机按此顺序确认）

1. **透明窗口**：背景透出桌面，无边框，最上层，鼠标穿透
2. **闲置动画**：宠物轻微上下浮动
3. **反应动画**：`curl -X POST http://127.0.0.1:4242/bark` → 摇尾 + 吠叫 + 气泡 + 音频
4. **LAN 可达**：LAN 内其他机器 `curl http://<你的IP>:4242/bark` 也能触发
5. **拖动**：Ctrl + 鼠标拖动，松开 Ctrl 恢复点击穿透
6. **Claude Code hook**：编码结束宠物自动叫

---

## 📎 进阶参考
- [`overview.md`](overview.md) — 详细架构、文件职责、出范围声明
- [`docs/cn-network-setup.md`](docs/cn-network-setup.md) — 国内网络环境搭建经验
- [`scripts/setup-cn-network.sh`](scripts/setup-cn-network.sh) — 一键配 cargo 镜像

---

## 📜 License
Private project.