#!/usr/bin/env bash
# 开发者便利入口；真正的安装/卸载逻辑与 App 设置页共用 yali-core。
set -euo pipefail
case "${1:-}" in
  "") ACTION=install ;;
  --uninstall) ACTION=uninstall ;;
  *) echo "用法：workflows/install-workflows.sh [--uninstall]" >&2; exit 2 ;;
esac
[ "$#" -le 1 ] || exit 2
BIN="${YALI_BIN:-$HOME/.yali/bin/yali}"
[ -x "$BIN" ] || {
  echo "请将构建后的 Yali.app 移入应用程序文件夹，并在设置 → 自动化中安装右键操作。" >&2
  echo "也可用 YALI_BIN 指向 App 内的 Contents/MacOS/yali。" >&2
  exit 1
}
exec "$BIN" services "$ACTION"
