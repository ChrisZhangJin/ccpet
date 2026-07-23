#!/usr/bin/env bash
# ccpet 国内网络环境一键配置
# 用途:在 ISP 限速 / Git 限流环境下,加速 Tauri + Rust + Node 工具链下载
# 用法:bash scripts/setup-cn-network.sh [--proxy http://host:port]
#
# 不带 --proxy:只配镜像源(适合无代理环境)
# 带 --proxy:同时把代理写到 git 全局配置(适合有可用代理)

set -euo pipefail

PROXY=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --proxy) PROXY="$2"; shift 2 ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

echo "==> [1/5] 备份现有 cargo / git / npm 配置"
mkdir -p ~/.ccpet-backup
[[ -f ~/.cargo/config.toml ]] && cp ~/.cargo/config.toml ~/.ccpet-backup/cargo-config.toml.bak && echo "    备份 ~/.cargo/config.toml"
git config --global --get http.proxy  >/dev/null 2>&1 && {
  git config --global http.proxy  > ~/.ccpet-backup/git-http.proxy.bak
  git config --global https.proxy > ~/.ccpet-backup/git-https.proxy.bak
  echo "    备份 git 全局代理"
} || echo "    (无 git 全局代理)"

echo "==> [2/5] 写入 cargo sparse 协议 + rsproxy + 调大 timeout"
mkdir -p ~/.cargo
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
echo "    ✓ ~/.cargo/config.toml"

echo "==> [3/5] npm 切 npmmirror"
npm config set registry https://registry.npmmirror.com
npm config set disturl  https://npmmirror.com/dist
echo "    ✓ $(npm config get registry)"

echo "==> [4/5] Homebrew bottles 切清华源"
if [[ "$(uname)" == "Darwin" ]]; then
  if [[ -n "${PROXY}" ]]; then
    echo "    (按 --proxy 处理,暂不动 brew)"
  fi
  BOTTLE_LINE='export HOMEBREW_BOTTLE_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles'
  if ! grep -q "HOMEBREW_BOTTLE_DOMAIN" ~/.zprofile 2>/dev/null; then
    echo "$BOTTLE_LINE" >> ~/.zprofile
    echo "    ✓ 已写入 ~/.zprofile(请 source ~/.zprofile 生效)"
  else
    echo "    (已配置,跳过)"
  fi
fi

echo "==> [5/5] 配置代理(可选)"
if [[ -n "${PROXY}" ]]; then
  git config --global http.proxy  "${PROXY}"
  git config --global https.proxy "${PROXY}"
  export http_proxy="${PROXY}" https_proxy="${PROXY}"
  echo "    ✓ git 全局代理 → ${PROXY}"
  echo "    ✓ 当前 shell 已 export(需重开终端或 source ~/.zshrc 持久化)"

  # 验证代理可达
  if curl -sI --connect-timeout 5 -x "${PROXY}" https://crates.io/ >/dev/null 2>&1; then
    echo "    ✓ 代理验证:可达 crates.io"
  else
    echo "    ⚠ 代理验证:无法连接,请检查 ${PROXY}"
  fi
else
  echo "    (无 --proxy 参数,跳过)"
  echo "    如有代理可执行: $0 --proxy http://host:port"
fi

cat <<'TIPS'

==> 完成!
==> 接下来:
   cd /Users/chris/Workspace/ccpet
   npm install
   npm run tauri:dev:mac

==> 失败排查:
   • 看到 'Waiting in queue...' → 镜像 Git 限流,换 sparse 协议
   • 看到 'spurious network error' → 调大 timeout / low-speed-limit
   • 看到 'Failed to connect to <ip>:<port>' → 代理客户端未启动

==> 详细复盘见:docs/cn-network-setup.md
TIPS
