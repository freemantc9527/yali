#!/usr/bin/env bash
# 本地运行：默认 Tauri；--web 仅启动浏览器演示。
set -euo pipefail

usage() {
  echo "用法：bash scripts/dev.sh [--web | --help]"
  echo "  默认启动原生桌面应用；--web 启动 http://localhost:5173 浏览器演示。"
}

MODE="native"
case "${1:-}" in
  --help|-h) usage; exit 0 ;;
  --web) MODE="web" ;;
  "") ;;
  *) usage >&2; exit 2 ;;
esac
[ "$#" -le 1 ] || { usage >&2; exit 2; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
command -v pnpm >/dev/null || { echo "缺少 pnpm，请先按 README 准备开发环境。" >&2; exit 1; }
[ -d "ui/node_modules" ] || { echo "请先运行 pnpm --dir ui install --frozen-lockfile。" >&2; exit 1; }

if [ "$MODE" = "web" ]; then
  exec pnpm --dir ui dev
fi
command -v cargo >/dev/null || { echo "缺少 Rust / cargo，请先按 README 准备开发环境。" >&2; exit 1; }
exec env -u CI cargo tauri dev
