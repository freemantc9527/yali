# T-003 · EngineRegistry 与 CLI 真实现

goal: yali-core 引擎层（Encoder trait + EngineRegistry + tokio 子进程池）+ CLI compress/convert/lossless/undo/doctor 真实现
context: docs/DESIGN.md §3（算法规格）/ §7（API 草图）；docs/BENCHMARKS.md（选型依据）
constraints:
  - 引擎全部走**子进程**（Phase A），不链接任何 GPL 库；不引入 imagequant-rs 等 crate（Phase B 期权，本轮不做）
  - tokio 从 workspace.dependencies 引回（`rt-multi-thread`/`process`/`macros`/`sync`），这是它在 Cargo.toml 里被注释掉的原因
  - 引擎路径：默认从 PATH + `(brew --prefix <name>)/bin` 探测，`YALI_ENGINES_PATH` 可覆盖；**mozjpeg 在 keg-only 路径，brew 的 cjpeg 是李鬼**
  - 缩放/元数据用 vips 子进程（冷启动 ~5s → 必须进程池预热，这是设计要求）
acceptance:
  - `cargo test -p yali-core engine` 绿（参数映射单测：level→quality 各格式按 DESIGN §3 公式）
  - `yali doctor` 输出每个引擎的路径/版本/可用性
  - `yali compress <合成样.jpg>` 产出 `-{profile}` 后缀文件，stdout JSON 摘要（input/output/bytes/engine/elapsed）
  - 负优化拦截：输出 ≥ 输入时不落盘，状态 `larger_output`（单测覆盖）
  - `yali undo <dir>` 按 `.yali-undo.json` 100% 还原（单测覆盖，真实文件走临时目录）
  - level 4 对 1MB 级样张耗时 < 1.5s（p50，冒烟即可，正式回归在 T-004）
estimated: 4h
manual-check: false

## 实现要点
- `EngineRegistry`：按 `Format` 注册引擎实例；`pipeline.engine.<fmt>` 可覆盖（Profile §2）
- level→参数映射 v1（DESIGN §3）：JPEG `quality = 100 - 10×level`；PNG level 1-2 oxipng 无损 / 3-10 pngquant 线性插值；GIF `colors = round((10-L)/9×223)+32`；WebP `quality = 100 - 5×level`
- 目标大小二分：≤5 轮，收敛 `[target-10KB, target]`，受 `minQualityFloor` 保护
- 无损归档：`cjxl --lossless_jpeg=1`（djxl 逐比特还原）
- `UndoManifest`：记录 input/output 对，写目标目录 `.yali-undo.json`
- 完成后：更新 STATE.md；benchkit 未建立前，质量数据先手测记录进 BENCHMARKS 备注区
