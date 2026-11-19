#!/usr/bin/env bash
# 一键构建 console 部署包（本机执行）：前端 dist + WSL musl 静态二进制 → prism-deploy.tar.gz
#
# 前置（已装好，勿重装）：
#   - WSL Ubuntu 内 rustup + musl target + musl std（标准库为 2026-11-12 手动解包装入）
#   - WSL /etc/resolv.conf 已改公共 DNS（223.5.5.5，WSL 网关 DNS 解析不了 CDN）
#   - ~/.cargo/config.toml 走 rsproxy-sparse
# 服务器端布局见 scripts/install.sh 头注释。
#
# 用法: bash scripts/deploy-console.sh
set -euo pipefail
cd "$(dirname "$0")/.."
REPO="$(pwd)"

echo ">> 1/3 前端构建（pnpm build）"
pnpm build

echo ">> 2/3 WSL musl 二进制（增量编译，只拷源码保 target 缓存）"
wsl -e bash -c '. $HOME/.cargo/env
for c in observer-server observer-storage; do
  mkdir -p ~/prism-build/$c
  cp -r /mnt/d/demo/rrweb-demo/crates/$c/src ~/prism-build/$c/
  cp /mnt/d/demo/rrweb-demo/crates/$c/Cargo.toml ~/prism-build/$c/
done
cd ~/prism-build/observer-server
cargo build --release --target x86_64-unknown-linux-musl 2>&1 | tail -3'

echo ">> 3/3 打包 prism-deploy.tar.gz"
wsl -e bash -c 'mkdir -p ~/prism-pack && cd ~/prism-pack && rm -rf dist
cp ~/prism-build/observer-server/target/x86_64-unknown-linux-musl/release/observer-server .
cp -r /mnt/d/demo/rrweb-demo/dist ./dist
cp /mnt/d/demo/rrweb-demo/scripts/install.sh . && sed -i "s/\r\$//" install.sh
chmod 755 install.sh observer-server
tar -czf /mnt/d/demo/rrweb-demo/prism-deploy.tar.gz observer-server dist install.sh
ls -lh /mnt/d/demo/rrweb-demo/prism-deploy.tar.gz'

echo
echo ">> 完成。后续三步："
echo "   1. scp $REPO/prism-deploy.tar.gz zhijian@192.168.17.108:/tmp/"
echo "   2. ssh 上服务器: sudo rm -rf /tmp/prism-deploy && mkdir /tmp/prism-deploy && sudo tar -xzf /tmp/prism-deploy.tar.gz -C /tmp/prism-deploy && sudo /tmp/prism-deploy/install.sh"
echo "   3. 验证: curl -s http://127.0.0.1:8080/whoami -H 'Authorization: Bearer sk_987654321'"
