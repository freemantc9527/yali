# T-004 · benchkit 合成金样与回归脚本

goal: benchkit/ 合成金样生成脚本 + bench.sh 三指标回归 + 基线入库
context: docs/DESIGN.md §9（金样回归口径）；docs/BENCHMARKS.md（历史实测，作对照）
constraints:
  - 金样必须**合成生成**（脚本可复跑），禁止用个人照片衍生物入仓库（合规红线）
  - 覆盖五类：照片（渐变+噪点+高细节纹理合成）/ UI 截图（纯色块+文字模拟）/ 动图（多帧 GIF）/ 透明 PNG / HEIC（用 sips 或 vips 生成）
  - `benchkit/local-samples/` 已在 .gitignore，个人素材只放这里
acceptance:
  - `benchkit/generate.sh` 一键生成全部合成样（输出进 benchkit/samples/，入库）
  - `benchkit/bench.sh` 跑完输出表格：每样×每引擎的 体积比 / SSIM（`magick compare -metric SSIM`）/ 耗时
  - 结果与 `benchkit/baseline.json` 比对，任一指标劣化 >3% 退出码非零
  - `baseline.json` 由 T-003 完成后的首次全量跑生成并入库
  - `cargo test` 不受影响（bench.sh 不进 cargo 测试回环，避免拖慢 <60s 预算）
estimated: 2h
manual-check: false

## 实现要点
- 生成工具只依赖本机已有的 CLI（ImageMagick / vips / sips），不写 Python 依赖
- bench.sh 参数：`--profile <name>`（默认 default）`--format <fmt>`；并发跑但计时取单图口径
- SSIM 计算注意：`magick compare` 的 metric 在 stderr；脚本要捕获处理
- 完成后：更新 STATE.md；此后所有压缩质量变更必须过 bench.sh
