# 国内网络环境下的 ccpet 开发环境搭建

> 2026-07-23 实战复盘。把 Tauri + Rust + Node 工具链在 ISP 限速 / Git 限流 / ghcr.io 抽风环境下的"加速配方"沉淀下来,避免每个新机器 / 新成员重走一遍。

## TL;DR(最快)

```bash
# 1) cargo:用 sparse 协议 + rsproxy + 调大 timeout(关键!)
cat > ~/.cargo/config.toml <<'EOF'
[registries.crates-io]
protocol = "sparse"

[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[http]
timeout = 120
low-speed-limit = 1
EOF

# 2) npm:npm 切 npmmirror
npm config set registry https://registry.npmmirror.com

# 3) Homebrew:bottles 切清华源
echo 'export HOMEBREW_BOTTLE_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles' >> ~/.zprofile
source ~/.zprofile

# 4) 有代理就 export(没有就跳过)
export http_proxy=http://<your-proxy>:port
export https_proxy=http://<your-proxy>:port
```

跑 `npm install && npm run tauri:dev:mac` 即可。

---

## 一、为什么走 sparse 协议(而非 git 协议)

crates 索引下载有两种协议:

| 协议 | 端点形态 | 限流策略 | 国内表现 |
|---|---|---|---|
| **git 协议** | `https://.../crates.io-index.git` | Git 服务端(GitLab/Gitea)按 IP + 频率限流,有"Waiting in queue"队列 | 大量被 Gitea 限流到 Position 几百甚至上千,5-10 分钟无响应 |
| **sparse 协议** | `sparse+https://.../index/`(普通 HTTPS REST) | 走 CDN,无限流机制 | 800KB/s ~ 几 MB/s,3 分钟拉完 |

**结论:国内环境直接选 sparse 协议**,不要碰 git 协议,除非在公司内网有企业级 git 代理。

`~/.cargo/config.toml` 必须显式开启 sparse(`Cargo 1.68+` 默认还是 git):

```toml
[registries.crates-io]
protocol = "sparse"
```

## 二、为什么调大 `low-speed-limit` / `timeout`

cargo 默认参数太严:

- `timeout = 30`(秒)
- `low-speed-limit = 10`(字节/秒)

ISP 偶发 30 秒内丢几个包,就被 cargo 判"网络挂了",报:

```
warning: spurious network error (3 tries remaining): process didn't exit successfully
```

调成:

```toml
[http]
timeout = 120
low-speed-limit = 1
```

宽容度大幅提升,基本不再假死。

## 三、镜像源怎么选

| 资源 | 推荐镜像 | 备选 | 不推荐 |
|---|---|---|---|
| crates.io 索引 | `sparse+https://rsproxy.cn/index/` | ustc sparse、tuna sparse | tuna **git 协议**(被自家 Gitea 限流) |
| crates.io crate 文件 | rsproxy.cn / rsproxy.cn/static.crates.io | ustc.crates | crates.io 官方(慢) |
| npm registry | `https://registry.npmmirror.com` | 阿里 `https://registry.niras.cn` | registry.npmjs.org(慢) |
| Homebrew bottles | `https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles` | 阿里 `mirrors.aliyun.com/homebrew/homebrew-bottles` | ghcr.io(国内经常抽风) |
| Homebrew git remote | `https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git` | 同上的 core / cask | github.com(队列限流) |
| Node.js 二进制 | npmmirror `https://npmmirror.com/mirrors/node` | 清华 tuna | nodejs.org(慢) |
| Xcode CLT | 走代理从 developer.apple.com 下载 .pkg | 走代理 | 直连(晚高峰极慢) |

## 四、代理到底要不要开

看情况:

- **有可用代理**(Clash / Surge / 路由器内置):**强烈建议开**。cargo 走代理后,crates 下载速度提升 5-10 倍。
- **没有代理**:`sparse + rsproxy + 调大 timeout` 已经能撑过首次索引拉取,只是偶尔抖动。

代理验证:

```bash
curl -sI --connect-timeout 5 -x http://<proxy>:<port> https://crates.io/
# 期望:HTTP/1.1 200 OK 或 HTTP/2 200
```

⚠️ **坑**:git CLI 也会走 `git config --global http.proxy` 指向的代理。如果代理挂了,git fetch 也会卡死,表现为 `Failed to connect to <ip>:<port>`。**每次换代理客户端时记得重新设 git 代理或清空**。

## 五、复盘:那天到底卡在哪儿

时间线(2026-07-23 23:00 ~ 00:25):

1. **Tauri 跨平台改造**完成,提交到 `feature/macos-cross-platform`
2. **`brew install node`**:报 `Warning: Bottle missing, falling back to the default domain`,卡在 ghcr.io
   - 解决:配清华 HOMEBREW_BOTTLE_DOMAIN
3. **`brew update`**:报 `Waiting in queue... (Position: 475)`
   - 这是清华 Gitea 限流(不是镜像慢)
   - 解决:`HOMEBREW_NO_AUTO_UPDATE=1 brew install node` 跳过 update
4. **Tauri dev 启动后,`Updating tuna index`**:卡在 20KB 几十分钟
   - 试 1:换 tuna 镜像的 git 协议 → Gitea 限流 Position 627
   - 试 2:换 sparse + rsproxy + 调大 timeout → 30 秒 25MB,起飞
5. **额外发现**:之前 git 全局代理 `http://192.168.32.101:18387` 误判为死地址,实际是当时代理客户端没开,后来开了。`git-fetch-with-cli = true` 才能让 cargo 复用 git 代理

## 六、最佳实践速查表

| 场景 | 配方 |
|---|---|
| 新机器第一次装 | 看 `scripts/setup-cn-network.sh`,一键跑 |
| cargo 下载卡死 | 先看 `~/.cargo/config.toml` 是不是 sparse 协议 + rsproxy |
| `Waiting in queue...` 队列 | 100% 是 Git 服务限流,换 sparse 协议 / 换镜像 |
| `spurious network error` | 调大 `[http] timeout / low-speed-limit` |
| 代理改了但 cargo 还连旧 IP | `pkill cargo` + `unset http_proxy/https_proxy` 重 export |
| npm install 慢 | `npm config set registry https://registry.npmmirror.com` |
| brew install 卡 ghcr.io | 配 HOMEBREW_BOTTLE_DOMAIN |
| Xcode CLT 装不上 | 走代理,或 developer.apple.com 直接下 .pkg |

## 七、给团队的建议

1. **新人入职文档**:把本文档放进 `docs/onboarding/`,新人第一周跑一遍 `scripts/setup-cn-network.sh`
2. **CI 镜像**:GitHub Actions / GitLab CI 跑器在国内时,**必须**配 `[http] timeout=120` + rsproxy,否则 CI 启动一次 cargo 要 30 分钟
3. **不要把代理地址写进仓库**:`192.168.32.101:18387` 这种是个人环境,文档只说"如果有代理 export 一下"
4. **优先 sparse 协议**:这是 Rust 1.68+ 的官方推荐协议,国内必选
5. **失败模式感知**:
   - `spurious network error` → timeout 太小 / 网络抖动
   - `Waiting in queue` → 镜像前端限流,换镜像或换协议
   - `Failed to connect to <ip>:<port>` → 代理客户端没开 / 配错
