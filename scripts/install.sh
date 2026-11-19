#!/usr/bin/env bash
# Prism console 升级脚本（服务器端执行）：替换二进制 + 前端产物，按原方式重启。
# 服务器布局（192.168.17.108，root 运行）：
#   二进制 /usr/local/bin/observer-server
#   静态   /opt/prism/web
#   数据   /var/lib/prism/recordings（本脚本不碰）
# 用法: sudo ./install.sh [token]  （缺省沿用既有 token）
set -euo pipefail

DIR="$(cd "$(dirname "$0")" && pwd)"
BIN=/usr/local/bin/observer-server
WEB=/opt/prism/web
DATA=/var/lib/prism/recordings
BIND=0.0.0.0:8080
TOKEN="${1:-sk_987654321}"

# 1. 旧进程是谁在管？systemd 单元 or 裸进程
PID="$(pgrep -f "^$BIN" | head -1 || true)"
UNIT=""
if [ -n "$PID" ] && [ -r "/proc/$PID/cgroup" ]; then
  UNIT="$(sed -n 's|.*/system.slice/\(.*\)\.service.*|\1|p' "/proc/$PID/cgroup" | head -1)"
fi

# 2. 停
if [ -n "$UNIT" ]; then
  echo ">> systemd 单元: $UNIT，stop"
  systemctl stop "$UNIT"
elif [ -n "$PID" ]; then
  echo ">> 非 systemd 托管 (PID=$PID)，kill"
  pkill -f "^$BIN" || true
  sleep 1
else
  echo ">> 未发现运行中的旧进程"
fi

# 3. 换文件（数据目录不动）
echo ">> 替换二进制 $BIN + 静态文件 $WEB"
install -m 755 "$DIR/observer-server" "$BIN"
rm -rf "$WEB"
cp -r "$DIR/dist" "$WEB"

# 4. 按原方式拉起
if [ -n "$UNIT" ]; then
  systemctl start "$UNIT"
  echo ">> 已 start $UNIT"
else
  nohup "$BIN" --bind "$BIND" --data-dir "$DATA" --token "$TOKEN" --web-dir "$WEB" \
    >> /var/log/observer-server.log 2>&1 &
  sleep 1
  pgrep -f "^$BIN" >/dev/null && echo ">> 已后台拉起，日志 /var/log/observer-server.log"
fi

echo
echo "验证: curl -s http://127.0.0.1:8080/whoami -H 'Authorization: Bearer $TOKEN'"
echo "浏览器: http://192.168.17.108:8080 （登录 token: $TOKEN）"
