#!/usr/bin/env bash
# T-013：校验 Yali.app 的 bundle 元数据（UTI 精确性 + 与 tauri.conf 不漂移）。
# 用法：scripts/bundle-check.sh [Yali.app 路径]
set -euo pipefail
cd "$(dirname "$0")/.."

APP="${1:-target/release/bundle/macos/Yali.app}"
PLIST="$APP/Contents/Info.plist"

[ -d "$APP" ] || { echo "✗ 找不到 $APP（先 cargo tauri build）"; exit 1; }
[ -f "$PLIST" ] || { echo "✗ 找不到 $PLIST"; exit 1; }
[ -x "$APP/Contents/MacOS/yali-app" ] || { echo "✗ 缺少可执行文件"; exit 1; }

python3 - "$PLIST" src-tauri/tauri.conf.json <<'PY'
import json, os, plistlib, sys

plist_path, conf_path = sys.argv[1], sys.argv[2]
with open(plist_path, "rb") as fh:
    plist = plistlib.load(fh)
with open(conf_path, encoding="utf-8") as fh:
    conf = json.load(fh)

fail = []

if plist.get("CFBundleIdentifier") != "app.yali":
    fail.append(f"CFBundleIdentifier = {plist.get('CFBundleIdentifier')!r}（期望 app.yali）")

docs = plist.get("CFBundleDocumentTypes") or []
if not docs:
    fail.append("CFBundleDocumentTypes 缺失")

plist_utis = {u for d in docs for u in d.get("LSItemContentTypes", [])}
declared = {
    u for fa in conf.get("bundle", {}).get("fileAssociations", []) for u in fa.get("contentTypes", [])
}
if plist_utis != declared:
    fail.append(
        "UTI 与 tauri.conf 的 fileAssociations 不一致："
        f"仅 plist {sorted(plist_utis - declared)} / 仅 conf {sorted(declared - plist_utis)}"
    )
if "public.image" in plist_utis:
    fail.append("出现了抽象 UTI public.image（会把「打开方式」扩大到所有图片格式）")

for d in docs:
    if not d.get("LSItemContentTypes"):
        fail.append(f"{d.get('CFBundleTypeName')} 缺 LSItemContentTypes")
    if d.get("CFBundleTypeRole") != "Viewer":
        fail.append(f"{d.get('CFBundleTypeName')} 的 role 不是 Viewer")
    if d.get("LSHandlerRank") != "Alternate":
        fail.append(f"{d.get('CFBundleTypeName')} 的 LSHandlerRank 不是 Alternate（可能抢默认打开）")

exts = {e.lower() for d in docs for e in d.get("CFBundleTypeExtensions", [])}
for ext in sorted({"png", "jpg", "jpeg", "heic", "heif", "gif", "webp", "avif", "jxl", "tif", "tiff", "bmp"}):
    if ext not in exts:
        fail.append(f"扩展名 {ext} 未声明")

if fail:
    print("✗ bundle 校验失败：")
    for item in fail:
        print("  -", item)
    sys.exit(1)

print(f"✓ bundle 校验通过：{len(docs)} 类图片，{len(plist_utis)} 个具体 UTI，无 abstract 类型")
print("  " + ", ".join(sorted(plist_utis)))
PY
python3 scripts/package-runtime.py check --app "$APP"
