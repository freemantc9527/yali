#!/usr/bin/env bash
# workflows/install-workflows.sh —— 安装 Finder 右键的鸭梨动作（三个，服务「分享」场景）
#
#   鸭梨 · 压一下      → compress --profile default（保持原格式，减体积）
#   鸭梨 · 转 JPG      → compress --profile jpg（兼容性最好，截图发谁都行）
#   鸭梨 · 压到 500K    → compress --profile mail（IM/邮件/表单有大小限制时）
#
# 用户级 .workflow（免 Xcode 免签名）；模板在 templates/quick-action.template/。
#
# 用法：
#   workflows/install-workflows.sh
#   workflows/install-workflows.sh --uninstall
#   YALI_BIN=/path/to/yali workflows/install-workflows.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
TEMPLATE="$ROOT/templates/quick-action.template"
SERVICES_DIR="$HOME/Library/Services"

# 动作表：标题 | bundle id | yali 子命令参数
ACTIONS=(
  "鸭梨 · 压一下|app.yali.quick-action.compress|compress \"\$@\" --profile default --quiet --notify"
  "鸭梨 · 转 JPG|app.yali.quick-action.to-jpg|compress \"\$@\" --profile jpg --quiet --notify"
  "鸭梨 · 压到 500K|app.yali.quick-action.shrink-500k|compress \"\$@\" --profile mail --quiet --notify"
)
# 历史遗留 bid（早期单动作版本），卸载与清理时一并处理
LEGACY_BIDS=("app.yali.quick-action")

flush_services() {
  /System/Library/CoreServices/pbs -flush 2>/dev/null || true
  killall pbs 2>/dev/null || true
  killall Finder 2>/dev/null || true
}

pbs_edit() { # $1=remove|register  $2...=bundle id 列表
  python3 - "$1" "${@:2}" <<'PYEN'
import plistlib, subprocess, sys

mode, bids = sys.argv[1], sys.argv[2:]
raw = subprocess.run(["defaults", "export", "pbs", "-"], capture_output=True, check=True).stdout
data = plistlib.loads(raw)
status = data.setdefault("NSServicesStatus", {})
ordering = data.setdefault("FinderOrdering", {})

# 清理：所有属于这些 bid 的条目（标题未知，按 bid 前缀匹配）
def is_ours(key: str) -> bool:
    body = key.removeprefix("SERVICE-")
    return any(body.startswith(f"{bid} - ") or body == bid for bid in bids)

if mode == "remove":
    removed = [k for k in list(status) + list(ordering) if is_ours(k)]
    for k in removed:
        status.pop(k, None)
        ordering.pop(k, None)
    print(f"  已清理 pbs 条目 {len(removed)} 条")
else:
    # 登记启用状态 + 排序。FinderOrdering 的值必须是 int：写成字符串会让 Finder
    # 解析整表失败并吞掉「快速操作」整栏（实测坑）。argv 形如 bid title bid title ...
    pairs = [(bids[i], bids[i + 1]) for i in range(0, len(bids) - 1, 2)]
    next_index = len(ordering)
    for bid, title in pairs:
        status_key = f"{bid} - {title} - runWorkflowAsService"
        ordering_key = f"SERVICE-{status_key}"
        status[status_key] = {
            "presentation_modes": {"ContextMenu": True, "FinderPreview": True, "ServicesMenu": True},
        }
        if ordering_key not in ordering:
            ordering[ordering_key] = next_index
            next_index += 1
    print(f"  已登记 {len(pairs)} 个动作的启用状态与排序（int）")

import tempfile, os
fd, path = tempfile.mkstemp(suffix=".plist")
with os.fdopen(fd, "wb") as f:
    plistlib.dump(data, f)
subprocess.run(["defaults", "import", "pbs", path], check=True)
os.unlink(path)
PYEN
}

if [ "${1:-}" = "--uninstall" ]; then
  BIDS=()
  TITLES=()
  for entry in "${ACTIONS[@]}"; do
    TITLES+=("${entry%%|*}")
    BIDS+=("${entry#*|}")
  done
  # bundle 名里带「 · 」，按标题精确删
  for title in "${TITLES[@]}"; do rm -rf "$SERVICES_DIR/$title.workflow"; done
  rm -rf "$SERVICES_DIR/鸭梨 · 压一下.workflow" # 早期单动作版本
  pbs_edit remove "${BIDS[@]}" "${LEGACY_BIDS[@]}"
  flush_services
  echo "已卸载全部鸭梨右键动作"
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
  echo "⚠ 使用 debug 构建（右键冷启动会慢）：先 cargo build --release -p yali-cli"
else
  echo "✗ 找不到 yali 可执行文件：先 cargo build -p yali-cli，或用 YALI_BIN=... 指定"
  exit 1
fi
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
[ -x "$BIN" ] || { echo "✗ 不可执行：$BIN"; exit 1; }

# 动作依赖的档案必须存在（default/jpg/mail 都在出厂六档里）
TMP_HOME="$(mktemp -d)"
YALI_HOME="$TMP_HOME" "$BIN" profiles list >/dev/null
for p in default jpg mail; do
  YALI_HOME="$TMP_HOME" "$BIN" profiles show "$p" >/dev/null || {
    echo "✗ 出厂档案 '$p' 缺失，中止安装"; rm -rf "$TMP_HOME"; exit 1;
  }
done
rm -rf "$TMP_HOME"

BIDS_FOR_PBS=()
for entry in "${ACTIONS[@]}"; do
  IFS='|' read -r TITLE BID COMMAND <<< "$entry"
  DEST="$SERVICES_DIR/$TITLE.workflow"
  rm -rf "$DEST"
  mkdir -p "$(dirname "$DEST")"
  cp -R "$TEMPLATE" "$DEST"
  python3 - "$DEST" "$BIN" "$TITLE" "$BID" "$COMMAND" <<'PY'
import os, sys
dest, bin_path, title, bid, command = sys.argv[1:6]
pairs = {
    "Contents/Info.plist": {"__BID__": bid, "__TITLE__": title},
    "Contents/document.wflow": {"__YALI_BIN__": bin_path, "__COMMAND__": command},
}
for rel, subs in pairs.items():
    path = os.path.join(dest, rel)
    text = open(path).read()
    for token, value in subs.items():
        assert token in text, f"{rel} 缺占位符 {token}"
        text = text.replace(token, value)
    open(path, "w").write(text)
PY
  plutil -lint "$DEST/Contents/Info.plist" >/dev/null
  plutil -lint "$DEST/Contents/document.wflow" >/dev/null
  BIDS_FOR_PBS+=("$BID|$TITLE")
  echo "✓ 已安装：$DEST"
done

# 清掉早期单动作版本的遗留登记，避免菜单里出现重复项
pbs_edit remove "${LEGACY_BIDS[@]}"
REGISTER_ARGS=()
for pair in "${BIDS_FOR_PBS[@]}"; do
  REGISTER_ARGS+=("${pair%%|*}" "${pair#*|}")
done
pbs_edit register "${REGISTER_ARGS[@]}"

flush_services
echo
echo "  yali 二进制：$BIN"
echo "  动作：压一下（default）/ 转 JPG（jpg）/ 压到 500K（mail）"
echo
echo "人工验收：Finder 选中图片 → 右键 → 快速操作 → 三条鸭梨"
echo "  · 没出现就去 系统设置 → 键盘 → 键盘快捷键 → 服务 勾选"
echo "  · 撤销：yali undo <目录>"
