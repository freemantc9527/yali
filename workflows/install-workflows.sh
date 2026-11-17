#!/usr/bin/env bash
# workflows/install-workflows.sh —— 安装 Finder 右键「鸭梨 · 压一下」
#
# 用户级 .workflow（免 Xcode 免签名）：模板在 workflows/templates/，
# 安装时渲染 yali 二进制绝对路径 → ~/Library/Services/ → flush pbs。
#
# 用法：
#   workflows/install-workflows.sh                # 自动找 yali（$YALI_BIN → PATH → target/{release,debug}）
#   workflows/install-workflows.sh --uninstall    # 卸载
#   YALI_BIN=/path/to/yali workflows/install-workflows.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
TEMPLATE="$ROOT/templates/yali-quick-action.workflow"
SERVICE_NAME="鸭梨 · 压一下"
BID="app.yali.quick-action"
DEST="$HOME/Library/Services/$SERVICE_NAME.workflow"

flush_services() {
  # 让 Finder 重新读取 Services 注册（pbs 有时自动重启，失败不致命）
  /System/Library/CoreServices/pbs -flush 2>/dev/null || true
  killall pbs 2>/dev/null || true
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -rf "$DEST"
  flush_services
  echo "已卸载：$DEST"
  exit 0
fi

if [ -n "${YALI_BIN:-}" ]; then
  BIN="$YALI_BIN"
elif command -v yali >/dev/null 2>&1; then
  BIN="$(command -v yali)"
elif [ -x "$REPO/target/release/yali" ]; then
  BIN="$REPO/target/release/yali"
elif [ -x "$REPO/target/debug/yali" ]; then
  BIN="$REPO/target/debug/yali"
  echo "⚠ 使用 debug 构建（右键冷启动会慢）：先跑 cargo build --release -p yali-cli"
else
  echo "✗ 找不到 yali 可执行文件：先 cargo build -p yali-cli，或用 YALI_BIN=... 指定"
  exit 1
fi
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
[ -x "$BIN" ] || { echo "✗ 不可执行：$BIN"; exit 1; }

# 渲染模板（python3 只用于安全替换路径里的 & / 特殊字符）
rm -rf "$DEST"
mkdir -p "$(dirname "$DEST")"
cp -R "$TEMPLATE" "$DEST"
python3 - "$DEST/Contents/document.wflow" "$BIN" <<'PY'
import sys
path, bin_path = sys.argv[1], sys.argv[2]
src = open(path).read()
assert "__YALI_BIN__" in src, "模板占位符缺失"
open(path, "w").write(src.replace("__YALI_BIN__", bin_path))
PY

plutil -lint "$DEST/Contents/Info.plist" >/dev/null
plutil -lint "$DEST/Contents/document.wflow" >/dev/null

# 启用并挂进 Finder 快速操作菜单（等价于在 系统设置→服务 里勾选）：
# 实测两点都不够——①只装 bundle + flush pbs 不上菜单；②用 `defaults write ... 7`
# 会把 FinderOrdering 的值写成**字符串**（其他条目都是 int），Finder 解析整个排序表
# 失败后直接吞掉「快速操作」整栏。因此必须用 plistlib 保类型读写。
python3 - "$BID" "$SERVICE_NAME" <<'PYEN'
import plistlib, subprocess, sys

bid, name = sys.argv[1], sys.argv[2]
status_key = f"{bid} - {name} - runWorkflowAsService"
ordering_key = f"SERVICE-{status_key}"

raw = subprocess.run(["defaults", "export", "pbs", "-"], capture_output=True, check=True).stdout
data = plistlib.loads(raw)

data.setdefault("NSServicesStatus", {})[status_key] = {
    "presentation_modes": {"ContextMenu": True, "FinderPreview": True, "ServicesMenu": True},
}
ordering = data.setdefault("FinderOrdering", {})
if ordering_key not in ordering:
    ordering[ordering_key] = len(ordering)  # int，接在现有条目之后

import tempfile, os
fd, path = tempfile.mkstemp(suffix=".plist")
with os.fdopen(fd, "wb") as f:
    plistlib.dump(data, f)
subprocess.run(["defaults", "import", "pbs", path], check=True)
os.unlink(path)
print(f"  已登记启用状态与排序（ordering={ordering[ordering_key]}，int）")
PYEN

flush_services

echo "✓ 已安装：$DEST"
echo "  yali 二进制：$BIN"
echo
echo "人工验收：Finder 里选中图片 → 右键 → 快速操作 →「${SERVICE_NAME}」"
echo "  · 首次在新系统上可能需要到 系统设置 → 键盘 → 键盘快捷键 → 服务 里勾选"
echo "  · 多选只发一条聚合通知；撤销：yali undo <目录>"
