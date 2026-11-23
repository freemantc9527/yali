#!/usr/bin/env bash
# benchkit/generate.sh —— 一键生成合成金样（全部由 CLI 合成，脚本可复跑）
#
# 依赖：ImageMagick (magick)、sips（系统自带）。不写 Python 依赖。
# 合规红线：禁止个人照片衍生物入库；个人素材只放 benchkit/local-samples/（已 gitignore）。
#
# 覆盖五类：照片（渐变+噪点+高细节纹理）/ UI 截图（色块+文字）/ 动图 GIF /
#          透明 PNG / HEIC（sips 编码）
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$ROOT/samples"
FONT="/System/Library/Fonts/Supplemental/Arial.ttf"   # ImageMagick 无默认字体，显式指定
mkdir -p "$OUT"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "▸ 照片类：渐变 + 结构纹理 + 中等噪声（1920×1280, JPEG q92；q75 时 SSIM≈0.97，贴近真实照片）"
magick -seed 42 -size 1920x1280 gradient:'#0f2d5c-#f2b134' \
  \( -size 1920x1280 plasma:fractal -blur 0x2 \) -compose overlay -composite \
  -attenuate 0.15 +noise Gaussian \
  -quality 92 "$OUT/photo.jpg"

echo "▸ UI 截图类：纯色块 + 文字模拟（1600×1000, PNG）"
magick -size 1600x1000 xc:'#14161a' \
  -fill '#1f2430' -draw 'rectangle 0,0 1600,64' \
  -fill '#2b3240' -draw 'rectangle 64,120 780,300' \
  -fill '#232833' -draw 'rectangle 820,120 1536,300' \
  -fill '#2b3240' -draw 'rectangle 64,340 1536,560' \
  -fill '#232833' -draw 'rectangle 64,600 780,920' \
  -fill '#2b3240' -draw 'rectangle 820,600 1536,920' \
  -font "$FONT" -fill '#e8eaf0' -pointsize 30 -annotate +28+42 'Yali · 参数控制台' \
  -fill '#9aa4b8' -pointsize 22 \
  -annotate +96+170 'Profile：default' \
  -annotate +96+215 '质量等级 4 · 自动格式' \
  -annotate +852+170 '拖入图片开始压缩' \
  -annotate +96+390 '任务列表 766KB → 334KB (-56%)' \
  -fill '#7c8699' -pointsize 18 \
  -annotate +96+680 '输出目录：sibling' \
  -annotate +852+680 '撤销清单：开' \
  "$OUT/ui.png"

echo "▸ 动图类：12 帧分形噪点（480×360, GIF）"
for i in $(seq 0 11); do
  magick -seed $((42 + i)) -size 480x360 "plasma:#ff5f6d-#ffc371" -blur 0x1 "$TMP/frame_$(printf '%02d' "$i").png"
done
magick -delay 8 -loop 0 "$TMP"/frame_*.png "$OUT/anim.gif"

echo "▸ 透明 PNG 类：圆形 + 半透明色块 + 文字（1000×1000, alpha）"
magick -size 1000x1000 xc:none \
  -fill '#ff7a00' -draw 'circle 500,440 500,140' \
  -fill '#2ec4b6cc' -draw 'roundrectangle 160,720 840,880 40,40' \
  -font "$FONT" -fill '#ffffffcc' -pointsize 56 -gravity center -annotate +0+0 'YALI' \
  -fill '#ffd166' -draw 'circle 760,300 760,210' \
  "$OUT/transparent.png"

echo "▸ 调色板 PNG 类：pngquant 产出的 Type:Palette（无 alpha，照片尺寸）"
# 这类文件是 libvips flatten 的雷区（无 alpha 还 flatten 会输出 graya 灰白图），
# 必须有样张守着「PNG → JPG」路径
# pngquant 只吃 PNG，先无损转一层中间图
magick "$OUT/photo.jpg" "$TMP/photo-src.png"
pngquant --quality 60-90 --speed 1 --force --output "$OUT/palette.png" "$TMP/photo-src.png"

echo "▸ HEIC 类：sips 编自照片样张（macOS 硬编）"
sips -s format heic "$OUT/photo.jpg" --out "$OUT/photo_heic.heic" >/dev/null

echo
echo "生成完毕 → $OUT"
ls -lh "$OUT" | tail -n +2 | awk '{printf "  %-16s %s\n", $9, $5}'
