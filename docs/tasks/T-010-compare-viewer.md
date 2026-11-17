# T-010 · 对比查看器（before/after 滑杆 + SSIM）

goal: 点任务行打开对比查看器：滑杆切分原图/压缩图，标注 SSIM（与 benchkit 同口径）
context: docs/DESIGN.md §5「对比查看器：Space 唤起，before/after 滑杆 + SSIM 标注」
constraints:
  - SSIM 口径必须与 benchkit 一致（ffmpeg ssim 滤镜；IM7 的 SSIM 已证实不可用）
  - 本地图片走 Tauri asset protocol（`protocol-asset` feature + security.assetProtocol scope）
  - 浏览器降级读不了本地路径：明确提示，不假装能看
acceptance:
  - `cargo test -p yali-core metrics` 绿（同图 SSIM=1；低质量副本 <0.95）
  - Tauri `ssim` command 注册；应用启动冒烟通过
  - ui 测试：clampPos、浏览器降级提示、无产出件可打开
  - 真机人工验收：滑杆拖动手感与 SSIM 数值（manual-check）
estimated: 3h
manual-check: true

## 已知取舍
- `security.assetProtocol.scope = ["**"]`：本地工具可读任意用户可读文件；M4 分发前评估收紧策略
