#!/usr/bin/env bash
# benchkit/bench.sh —— 合成金样三指标回归（体积比 / SSIM / 耗时）
#
# 用法：
#   benchkit/bench.sh                       # 默认 --profile default
#   benchkit/bench.sh --profile web         # 换档
#   benchkit/bench.sh --format webp         # 临时覆盖输出格式（不动磁盘档案）
#   benchkit/bench.sh --update-baseline     # 首次全量跑后重写 baseline.json
#
# 判定（DESIGN §9）：体积比 / SSIM 劣化 >3% 退出码非零；耗时仅观察。
# level≥5 档照片类额外要求 SSIM≥0.95。
# 不进 cargo 单测回环；质量相关变更必须过本脚本。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$ROOT/.." && pwd)"
SAMPLES="$ROOT/samples"
BASELINE=""  # 解析参数后按档位定为 baseline-<profile>.json
YALI_BIN="${YALI_BIN:-$REPO/target/debug/yali}"

PROFILE="default"
FORMAT=""
RUNS=3
UPDATE_BASELINE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --profile) PROFILE="$2"; shift 2 ;;
    --format) FORMAT="$2"; shift 2 ;;
    --runs) RUNS="$2"; shift 2 ;;
    --update-baseline) UPDATE_BASELINE=1; shift ;;
    *) echo "未知参数: $1"; exit 2 ;;
  esac
done

SAMPLES_LIST=(photo.jpg ui.png palette.png anim.gif transparent.png photo_heic.heic)
for s in "${SAMPLES_LIST[@]}"; do
  [ -f "$SAMPLES/$s" ] || { echo "✗ 缺样张 benchkit/samples/$s（先跑 benchkit/generate.sh）"; exit 2; }
done

if [ ! -x "$YALI_BIN" ]; then
  echo "▸ 构建 yali CLI…"
  (cd "$REPO" && cargo build -p yali-cli -j4 >/dev/null)
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# YALI_HOME 隔离到临时目录，避免污染真实档案；--format 用临时 patch 档实现
export YALI_HOME="$WORK/yali-home"
mkdir -p "$YALI_HOME/profiles"
"$YALI_BIN" profiles list >/dev/null # 触发出厂播种
USED_PROFILE="$PROFILE"
if [ -n "$FORMAT" ]; then
  "$YALI_BIN" profiles show "$PROFILE" > "$WORK/src-profile.json"
  python3 - "$WORK/src-profile.json" "$YALI_HOME/profiles/bench-fmt.json" "$FORMAT" <<'PY'
import json, sys
src, dst, fmt = sys.argv[1], sys.argv[2], sys.argv[3]
p = json.load(open(src))
p["name"] = "bench-fmt"
p["format"]["output"] = fmt
json.dump(p, open(dst, "w"), ensure_ascii=False, indent=2)
PY
  USED_PROFILE="bench-fmt"
fi

level_of_profile() {
  python3 - "$YALI_HOME/profiles/$USED_PROFILE.json" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
q = p["quality"]
print("recommended" if q["kind"] == "recommended" else q["level"] if q["kind"] == "level" and q["level"] is not None else -1)
PY
}
LEVEL="$(level_of_profile)"

# SSIM：ffmpeg 的 ssim 滤镜（IM7.1.2 构建的 compare -metric SSIM 语义损坏：
# 同像素输入输出 0 而非 1，不可用——见 benchkit/README.md 偏离说明）。
# 先把两侧统一解码成 PNG（magick 解不了 HEIC 时走 sips），再比；动图取首帧。
# SSIM：ffmpeg 的 ssim 滤镜（IM7.1.2 构建的 compare -metric SSIM 语义损坏：
# 同像素输入输出 0 而非 1，不可用——见 benchkit/README.md 偏离说明）。
# 优先让 ffmpeg 直接解码两侧原文件（标准口径）；HEIC 等 ffmpeg 解不了的容器，
# 先统一转成白底压平的 PNG 再比（动图取首帧）。
measure_ssim() {
  ffmpeg -hide_banner -loglevel info -i "$1" -i "$2" -lavfi ssim -f null - 2>&1 \
    | grep -o 'All:[0-9.]*' | head -1 | cut -d: -f2
}
to_png() { # $1 源，$2 目标：白底压平——透明件的 alpha 在 SSIM 里是伪影来源
  magick "$1[0]" -background white -alpha remove -alpha off "$2" >/dev/null 2>&1 && return 0
  sips -s format png "$1" --out "$2.raw.png" >/dev/null 2>&1 || return 1
  magick "$2.raw.png" -background white -alpha remove -alpha off "$2" >/dev/null 2>&1
}
has_alpha() {
  magick identify -format '%[channels]' "$1[0]" 2>/dev/null | grep -q 'a'
}
ssim_between() {
  local a="$1" b="$2" v
  # 含 alpha 的图先压平到白底（alpha 差异是 SSIM 伪影，不做视觉质量信号）
  if has_alpha "$a" || has_alpha "$b"; then
    local pa="$WORK/ssim_a.png" pb="$WORK/ssim_b.png"
    to_png "$a" "$pa" || { echo ""; return; }
    to_png "$b" "$pb" || { echo ""; return; }
    v="$(measure_ssim "$pa" "$pb")"
  else
    v="$(measure_ssim "$a" "$b")"
  fi
  if ! [[ "$v" =~ ^[0-9]*\.?[0-9]+$ ]]; then
    local pa="$WORK/ssim_a.png" pb="$WORK/ssim_b.png"
    to_png "$a" "$pa" || { echo ""; return; }
    to_png "$b" "$pb" || { echo ""; return; }
    v="$(measure_ssim "$pa" "$pb")"
  fi
  [[ "$v" =~ ^[0-9]*\.?[0-9]+$ ]] && echo "$v" || echo ""
}

BASELINE="$ROOT/baseline-$PROFILE.json"
echo "▸ bench：profile=$USED_PROFILE runs=$RUNS level=$LEVEL baseline=$(basename "$BASELINE")"
printf "%-18s %-10s %12s %12s %8s %8s %8s\n" "sample" "engine" "in" "out" "ratio" "ssim" "ms(med)"
printf '%.0s─' {1..80}; echo

RESULTS="$WORK/results.jsonl"
: > "$RESULTS"
for sample in "${SAMPLES_LIST[@]}"; do
  src="$SAMPLES/$sample"
  declare -a times=()
  out_path=""
  out_bytes=""
  engine=""
  status=""
  for r in $(seq 1 "$RUNS"); do
    mkdir -p "$WORK/run$r"
    cp "$src" "$WORK/run$r/$sample"
    json="$("$YALI_BIN" compress "$WORK/run$r/$sample" --profile "$USED_PROFILE")"
    read -r t o ob en st < <(python3 - "$json" <<'PYP'
import json, sys
r = json.loads(sys.argv[1])
print(r.get("elapsedMs", 0), r.get("output") or "-", r.get("outputBytes") or 0,
      r.get("engine", "-"), r.get("status", "-"))
PYP
)
    times+=("$t")
    out_path="$o"; out_bytes="$ob"; engine="$en"; status="$st"
  done
  med="$(printf '%s\n' "${times[@]}" | sort -n | awk '{a[NR]=$1} END {print a[int((NR+1)/2)]}')"
  in_bytes="$(stat -f%z "$src")"
  ratio=""; ssim=""
  if [ "$out_path" != "-" ] && [ -f "$out_path" ]; then
    ratio="$(python3 -c "print(f'{$out_bytes/$in_bytes:.4f}')")"
    ssim="$(ssim_between "$src" "$out_path" || true)"
  fi
  [ -n "$ssim" ] || ssim="n/a"
  printf "%-18s %-10s %12s %12s %8s %8s %8s\n" \
    "$sample" "$engine" "$in_bytes" "$out_bytes" "$ratio" "$ssim" "$med"
  python3 - "$RESULTS" "$sample" "$engine" "$status" "$in_bytes" "$out_bytes" "$ratio" "$ssim" "$med" <<'PYP'
import json, sys
path, sample, engine, status, inb, outb, ratio, ssim, med = sys.argv[1:]
rec = {
    "sample": sample, "engine": engine, "status": status,
    "inputBytes": int(inb),
    "outputBytes": int(outb) if outb not in ("", "0") else None,
    "ratio": float(ratio) if ratio else None,
    "ssim": None if ssim == "n/a" else float(ssim),
    "msMedian": int(med),
}
with open(path, "a") as f:
    f.write(json.dumps(rec, ensure_ascii=False) + "\n")
PYP
done

# 汇总 + 基线比对
python3 - "$RESULTS" "$BASELINE" "$UPDATE_BASELINE" "$USED_PROFILE" "$RUNS" "$LEVEL" <<'PY'
import json, os, sys, datetime

results_path, baseline_path, update, profile, runs, level = sys.argv[1:]
records = [json.loads(l) for l in open(results_path) if l.strip()]

failures = []
observations = []
# 质量门：level≥5 时照片类样本 SSIM≥0.95（DESIGN §9 口径；平涂图形/动图/HEIC 的
# SSIM 对边缘 ringing 极敏感，实测 q75 webp 也只有 0.86~0.92 而 PSNR>50dB，
# 绝对门只对照片类有意义——细节见 benchkit/README.md）
lvl = -1 if level == "recommended" else int(level)
PHOTO_SAMPLES = ("photo.jpg",)
if lvl >= 5 or level == "recommended":
    for r in records:
        if r["sample"] not in PHOTO_SAMPLES or r["ssim"] is None:
            continue
        if r["ssim"] < 0.95:
            failures.append(f"{r['sample']}: SSIM {r['ssim']:.4f} < 0.95（level {lvl} 照片类质量门）")

doc = {
    "generatedAt": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
    "profile": profile,
    "runsPerSample": int(runs),
    "records": records,
}

if update == "1":
    with open(baseline_path, "w") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"\n▸ baseline 已写入 {os.path.relpath(baseline_path)}")
elif os.path.exists(baseline_path):
    base = json.load(open(baseline_path))
    if base.get("profile") != profile:
        print(f"\n▸ 基线档位为 {base.get('profile')}，本次 {profile}：跳过基线比对")
    else:
        old = {r["sample"]: r for r in base["records"]}
        print("\n▸ 基线比对（体积比 / SSIM 劣化阈值 3%；耗时仅观察）")
        for r in records:
            o = old.get(r["sample"])
            if not o:
                print(f"  ⚠ {r['sample']}: 基线无记录")
                continue
            def worse(name, new, oldv, lower_better, target=failures):
                if new is None or oldv in (None, 0):
                    return
                delta = (oldv - new) / oldv if lower_better else (new - oldv) / oldv
                if delta > 0.03:
                    target.append(f"{r['sample']}: {name} 劣化 {delta*100:.1f}%")
            worse("体积比", r["ratio"], o["ratio"], lower_better=False)
            worse("SSIM", r["ssim"], o["ssim"], lower_better=True)
            worse("耗时", r["msMedian"], o["msMedian"], lower_better=False, target=observations)
        if not failures:
            print("  ✓ 体积比 / SSIM 均在阈值内")
else:
    print("\n▸ 无 baseline.json：本次为首次全量跑，请用 --update-baseline 入库")

if observations:
    print("\n▸ 耗时观察（受环境影响，不影响退出码）：")
    for observation in observations:
        print(f"  - {observation}")

if failures:
    print("\n✗ 回归未通过：")
    for f in failures:
        print(f"  - {f}")
    sys.exit(1)
print("\n✓ bench 通过")
PY
