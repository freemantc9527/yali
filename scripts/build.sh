#!/usr/bin/env bash
# 构建本地 release .app，随后校验 bundle 元数据。
set -euo pipefail

usage() {
  echo "用法：bash scripts/build.sh [--help]"
  echo "  构建前端和 Yali.app，再运行 bundle-check；支持 CARGO_TARGET_DIR。"
}
case "${1:-}" in
  --help|-h) usage; exit 0 ;;
  "") ;;
  *) usage >&2; exit 2 ;;
esac
[ "$#" -eq 0 ] || { usage >&2; exit 2; }

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
for tool in cargo pnpm python3; do
  command -v "$tool" >/dev/null || { echo "缺少 ${tool}，请先按 README 准备开发环境。" >&2; exit 1; }
done
[ -d "ui/node_modules" ] || { echo "请先运行 pnpm --dir ui install --frozen-lockfile。" >&2; exit 1; }

# tauri.conf.json 的 beforeBuildCommand 负责生成真实 ui/dist，避免重复构建。
BUILD_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
if [[ "$BUILD_DIR" != /* ]]; then
  BUILD_DIR="$ROOT/$BUILD_DIR"
fi
CARGO_TARGET_DIR="$BUILD_DIR" cargo build --release -p yali-cli
python3 "$ROOT/scripts/package-runtime.py" prepare --build-dir "$BUILD_DIR"
CARGO_TARGET_DIR="$BUILD_DIR" env -u CI cargo tauri build --bundles app --config "$BUILD_DIR/yali-bundle-input/tauri-bundle.json"
APP="$BUILD_DIR/release/bundle/macos/Yali.app"
python3 "$ROOT/scripts/package-runtime.py" finalize --build-dir "$BUILD_DIR" --app "$APP"
bash "$ROOT/scripts/bundle-check.sh" "$APP"
echo "构建完成：$APP"
