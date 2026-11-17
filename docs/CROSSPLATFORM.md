# CROSSPLATFORM · 跨平台可行性分析（方向备忘，不排期）

> 决策顺序：macOS 做透 → Linux → Windows。架构先天有利（Rust core + Tauri UI 本就跨平台），专属逻辑集中在集成层与系统白嫖能力两块。

## 1. 分层可移植性

| 层 | 可移植性 | 说明 |
|---|---|---|
| yali-core（Profile/Pipeline/TaskQueue/Undo/Metrics） | ★★★★★ | 纯 Rust + tokio，零平台代码 |
| 引擎层（子进程） | ★★★★☆ | mozjpeg/pngquant/oxipng/gifsicle/libwebp/libavif/libjxl 全平台可用 |
| GUI（React） | ★★★★★ | Tauri 三平台官方支持 |
| 系统集成（右键/剪贴板/watch/通知） | ★★☆☆☆ | 每平台一套胶水，工作量集中区 |
| 系统白嫖（ImageIO/Vision） | ★☆☆☆☆ | macOS 独占，需第三方替代 |

**总体：约 80% 代码可复用；成本集中在集成层（~15%）与白嫖替代（~5%）。**

## 2. 能力替代对照

| 能力 | macOS | Linux | Windows |
|---|---|---|---|
| 右键快速操作 | .workflow | Nautilus/KDE 服务脚本（desktop entry + shell，纯文本） | Win11 上下文菜单（Shell 扩展，三者中最别扭；可先"打开方式+拖拽"） |
| HEIC 输入/输出 | ImageIO 硬编 | libheif（LGPL 动态 OK）+ x265 软编慢；**HEVC 专利费需声明，默认关** | Media Foundation / libheif |
| JXL 解码 | ImageIO 原生 | libjxl 自带 | libjxl 自带 |
| 抠图 | Vision | rembg / u2net ONNX（Apache-2.0 可捆绑） | 同 Linux |
| OCR | Vision | Tesseract / RapidOCR（中文强） | Win OCR API / RapidOCR |
| 剪贴板/watch/托盘/通知 | ✅ | arboard/notify/tray 全跨平台 | ✅ |

## 3. 分发差异

- **Linux**：AppImage 捆绑引擎为主，发行版包管理器为辅
- **Windows**：引擎 MSVC 构建链维护成本高——首发可"用户自备 CLI"（`YALI_ENGINES_PATH`）
- **CI**：GitHub Actions 三平台矩阵；macOS runner 保持 CLT-only 命令集（与本地一致）

## 4. 路线建议

```
macOS（当前）→ Linux（开源社区用户重合最大、集成纯文本、rembg 生态成熟）→ Windows（量最大但集成/构建最贵）
```

## 5. 现在就该做的预埋（零成本，写进架构约定）

1. 平台代码只允许出现在 `yali-integration` crate 的 trait 实现；yali-core 禁止散落的 `#[cfg(target_os)]`
2. 引擎发现三级：`YALI_ENGINES_PATH` → 捆绑目录 → 系统 PATH
3. 路径全部走 `dirs` crate
4. 文档从第一天提供 Linux from-source 安装段
