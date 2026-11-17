# 鸭梨 Yali

> **让图片没有压力** · *No pressure for your pictures.*

macOS 原生的图片压缩与处理入口：右键即压、后台静默、Profile 驱动；需要精调时 GUI 里全是旋钮。Tauri 2 + Rust 内核，全本地处理，无遥测。

## 数据说话（M1 Pro 实测，同画质口径）

| 场景 | 能力 | 相对原图 |
|---|---|---|
| JPEG 重编码 | mozjpeg 主力 | -58% |
| 网页输出 | WebP / AVIF | -69% / **-78%** |
| 无损归档 | JXL 包裹，可逐比特还原 | **-17.2%，零损失** |
| PNG | pngquant + oxipng 组合 | -51% |
| 动图 | 转 Animated WebP | -24% |
| HEIC | 系统硬编硬解 | -42%，112ms |

明细与口径：`docs/BENCHMARKS.md`。

## 特性规划

- 🗜 压缩/转换：JPEG · PNG · WebP · AVIF · JPEG XL · HEIC · GIF · SVG
- 🎯 目标大小二分收敛（带画质下限）· 负优化拦截 · 撤销清单
- 🖱 入口：Finder 快速操作「压一下」/ 拖 Dock / 剪贴板监听 / watch 文件夹 / CLI `yali`
- 🔍 智能：照片 vs 截图内容分类路由；Vision 抠图与 OCR（纯本地）
- 🛡 隐私：EXIF/GPS 一键清洗；全本地处理承诺
- ⚙️ 一个事实源：所有入口消费同一份 Profile JSON，GUI 调参右键即生效

## 状态

**M-1 完成（CLT-only 构建终验通过），M0 内核开发进行中。**

| 里程碑 | 内容 |
|---|---|
| M-1 ✅ | 仓库/骨架/工具链终验 |
| M0 | Rust 内核 + CLI（压缩/转换/无损归档/目标大小/undo） |
| M0.5 | Finder 快速操作「压一下」 |
| M1 | Tauri GUI 参数控制台 |
| M2 | 剪贴板 / watch / 托盘 |
| M3 | 分类路由 / Vision 抠图 / OCR / 标定表 |
| M4 | Release + brew tap + 自动更新 |

## 文档地图

| 文档 | 内容 |
|---|---|
| `docs/DESIGN.md` | 产品与工程规格（设计权威） |
| `docs/BENCHMARKS.md` | 引擎实测数据与选型 |
| `docs/TOOLCHAIN.md` | 工具链硬约束（无 Xcode）与实证边界 |
| `docs/NAMING.md` | 命名注册表 |
| `docs/WORKFLOW.md` | 任务卡制与验收流程 |
| `docs/CROSSPLATFORM.md` | 跨平台备忘 |
| `docs/STATE.md` | **当前进度（新 session 先读这个）** |

## 开发

```bash
brew bundle --file Brewfile
cargo test                        # 内核测试
cargo run -p yali-cli -- profiles show default
```

工程约定见 `AGENTS.md`（人类与 AI Agent 同等适用）。

## License

Apache-2.0
