# DESIGN · 产品与工程规格

当前进度见 [STATE](STATE.md)，界面行为与断点以 [UI-REDESIGN](UI-REDESIGN.md) 为准。本文区分已实现能力与后续方向。

## 1. 产品定义

Yali 是 macOS 优先的本地图片压缩与转换工具：Finder 快速操作静默使用已保存预设，GUI 支持本批调参与结果对比，CLI 提供可脚本化入口。所有入口共用 Rust 内核与 Profile JSON，无账户、云端或遥测。

原则：Profile 统一参数；右键永不询问、精调归 GUI / CLI；保留原图；原生 arm64；开发仅依赖 CLT，无 Xcode。失败件持久化尚未进入当前承诺范围。

只处理图片，不提供音视频处理。macOS 保留完整图片能力；Windows 实验 ZIP 已通过云端编译、Rust/UI 测试、包内图片引擎与 DLL 检查、最终 ZIP 的六类输入和质量对照；Windows 11 GUI 仍待实机验收，具体边界见 [CROSSPLATFORM](CROSSPLATFORM.md)。

## 2. 数据契约

Profile 的完整字段与默认档见 [Schema](../schema/profile.schema.json) 和 [fixtures](../fixtures/)；存储于 `~/.yali/profiles/*.json`，camelCase、拒绝未知字段、原子写。`YALI_HOME` 可重定向整个数据根。

四档按显示顺序：default（日常压缩）、jpg（转 JPG）、thumbnail（缩小尺寸，最长边 1024px，锁定宽高比、不放大小图）、archive（无损归档，JPEG 的 JXL 无损包裹）。thumbnail 保留内部标识以兼容已有引用，上一版未修改的 320px 默认档读取时升级，用户改过的预设保留。Finder 只绑定 default / jpg。旧版 web/mail/screenshot 原版不再列为预设，修改过的档保留为自定义预设。GUI 本批草稿不自动写盘，明确保存后才影响后续入口；CLI / workflow 下次调用读取新文件。

default / jpg / thumbnail 使用 `quality.kind=recommended`：JPEG q75 与 jpegtran 无损候选择小；PNG quality 94 到 98 / speed 1 后接 oxipng -o 2；已量化或动画 PNG 走无损优化，量化明确退出 99 时回退无损。只读升级完全匹配旧出厂配置的档，不改写旧文件；自定义档与手动 level / targetBytes 语义保留。推荐模式不使用 level 字段，保留其值便于切换手动模式。GUI 的质量参数位于高级折叠，推荐档默认关闭。细节以当前内核实现和 [BENCHMARKS](BENCHMARKS.md) 为准。

Schema 字段不等于已实现能力：metadata.exif/icc、pipeline.classify、resize.cropAnchor、output.largerPolicy 的完整执行效果尚未验证；GUI 保留值，不增加保证生效的控件。内容分类与智能裁剪属 M3。

应用配置独立存于同根 `settings.json`：schemaVersion=2，fontScalePercent=100（只接受 80 / 100 / 120 / 150 / 180 / 200 六档）；默认 clipboard=false、minBytes=65536；watch=false、dirs=[]、settleMs=800；tray=true、notify=true。v1 读取时补齐字号、升级内存版本；旧的范围内连续字号归入最近固定档，下次保存才写回。缺文件 / 缺字段用默认；语法坏先备份 `.bad-<毫秒>` 再回落；未知字段、类型错误或未来版本报错且不覆盖。GUI 显式保存后应用。

应用数据统一存于 `~/.yali/`：`profiles/`、`settings.json`、`data/undo/`、`cache/previews/`、`cache/clipboard/`、`cache/tmp/`；后续文件日志也必须使用该根。首次访问复制旧 `~/Library/Application Support/app.yali/` 的缺失文件，保留旧目录、不覆盖新文件、不迁移可再生缓存；先完整写入同分区临时文件并同步，再以不覆盖目标的原子操作发布，复制中断后可重试。图片产物仍按预设输出；产物的原子写入临时文件与产物同目录，完成即重命名，不作为应用数据保留。撤销清单按输出目录区分，兼容读取旧的图片旁清单。迁移和数据边界以当前实现与 [ARCHITECTURE](ARCHITECTURE.md) 为准。

WebView 使用非持久 WebKit 数据存储；设置和预设持久化走 core，不依赖网页 localStorage。旧 Library 数据的本机清理按明确清单执行，验证复制完整后才删除；macOS 自身可能维护文件选择器偏好等系统文件，不作为应用业务设置使用。

## 3. 压缩管道与算法规格

```
读文件 → 魔数嗅探真实格式 → 预缩放(照片类) → 按格式压缩 → 格式转换(如需)再压 → 负优化判定 → 命名落盘
```

- **level→参数映射 v1**（出厂标定表，`calibration.json` 由 M3 参数网格扫描替换）：
  - JPEG：mozjpeg `quality = 100 - 10×level`
  - PNG：level 1-2 走 oxipng 无损；3-10 走 pngquant，speed/quality 按 level 线性插值（M3 换 SSIM 目标点标定：{0.99…0.90} 十档）
  - GIF：gifsicle `colors = round((10-L)/9×223)+32`，optimizationLevel 3
  - SVG：svgo 默认插件集
  - WebP 输出：`quality = 100 - 5×level`
- **recommended** 使用上述固定 JPEG / PNG 参数，其他格式沿用旧 level 4。候选与主产物均验证格式和尺寸；检测到外部 FFmpeg 时增加解码校验，没有 FFmpeg 仍继续图片处理。无损候选只基于实际编码输入，避免取消缩放。jpegtran 保留原有标记，完整元数据策略仍按 §2 的未验证边界记录；可选优化失败保留有效主产物，主编码失败报错。
- **目标大小二分**：`target = KB×1000`；初始比例 `min(target/size, 1)`；≤5 轮二分（偏大记上界/偏小记下界取中点），收敛区间 `[target-10KB, target]`；受 `minQualityFloor` 保护
- **负优化拦截**：输出 ≥ 输入 → 不落盘，状态 `larger_output`，UI 标 ⚠
- **无损归档**：`cjxl --lossless_jpeg=1` 包裹，`djxl` 可逐比特还原原 JPEG
- **动图策略**：默认保守（保持 GIF）；`animated: "webp"` 显式开启（实测 -24%）
- **内容分类路由**（M3）：色方图直方图 + 边缘密度启发式，照片→有损管道 / UI 截图→无损 WebP / 透明→pngquant 或 oxipng


跨格式转换允许体积增加；同格式负优化不落盘。无损归档的逐比特还原保证仅适用于已支持的 JPEG 包裹路径，不能推广到所有格式组合。

## 4. 入口矩阵

| 入口 | 当前实现与边界 |
|---|---|
| Finder 快速操作 | 两条用户级 `.workflow`：「压一下 / 转 JPG」；具体 UTI 按动作声明，JPEG 隐藏转 JPG，CLI 聚合通知 |
| Dock / 打开方式 | CFBundleDocumentTypes + RunEvent::Opened + 冷启动缓冲；GUI 使用 default 快照自动排队 |
| GUI | 手动添加 / 拖入形成 pending，点击开始提交本批草稿 |
| 剪贴板 | NSPasteboard 主线程轮询，changeCount + 内容摘要去重，默认关，输出下载目录 |
| watch | notify/FSEvents，settle 防抖 + (mtime, size) 去重 + 产物 ignored，默认关 |
| 托盘 | 关窗常驻 / 唤回；托盘拖入未实现 |
| CLI | compress / convert / lossless / undo / profiles / settings / doctor |

NSServices 为备选方向，未实现。GUI 不汇集 CLI / watch / 剪贴板的全部任务历史。

## 5. GUI

响应式工作台、预设管理、设置与独立对比视图已经实现，部分原生交互仍待人工验收。用户已确认效果图、shadcn/ui（Radix）、Tailwind CSS 4 与 Lucide，保留 React / TypeScript / Vite / Tauri。

完整行为、状态机、接口、视觉与响应式规则只维护在 [UI-REDESIGN](UI-REDESIGN.md)，不在本文重复。

## 6. 当前架构

| 目录 | 职责 |
|---|---|
| `crates/yali-core` | Profile / Settings Store、Pipeline、EngineRegistry、命名、undo、SSIM |
| `crates/yali-cli` | clap 命令门面，安装命令名 yali |
| `src-tauri` | commands、batch、preview、open-file、托盘、剪贴板、watch；capabilities 与精确 Info.plist |
| `ui` | React / Vite / shadcn / Tailwind，bridge 集中原生调用，reducer 管理会话批次 |
| `scripts` / `workflows` | dev / build / bundle-check；两条 Finder 动作的模板与安装 |
| `benchkit` / `schema` / `fixtures` | 合成质量回归、数据契约、四档出厂预设与升级识别样本 |

引擎当前通过子进程调用，GUI 批内并发 `min(cores-1,4)`。预览只生成应用缓存副本；原图与真实结果路径不被预览替换。

macOS 图片处理引擎自包含，App 不再携带 FFmpeg。SSIM 依赖可选外部 FFmpeg；设置页可查看路径、检查 SSIM 滤镜并重新检查，未找到或不可用时隐藏分数。结果仅在会话内复用，不写入 settings.json；视觉对比仍可使用。

尚未创建：`helpers/`、`yali-integration`、`yali-vision`。SQLite 队列 / metrics、GUI 撤销、updater、库内引擎链接均为后续方向，不是当前实现。

## 7. 接口定位

真实类型与函数以代码为准：core 的 profile / pipeline / engines / undo / metrics；CLI 的 main；Tauri commands 与 batch / preview；前端 bridge。定位前按 [AGENTS](../AGENTS.md) 使用 CodeGraph。GUI 新 command 合同见 UI 规格，不维护容易漂移的伪代码 API。

## 8. 里程碑

工具链 → 内核 / CLI → Finder → GUI → 入口与队列 → 智能与标定 → 发布。当前完成度与人工待验统一见 [STATE](STATE.md)，开发过程和验收证据留在本地 `.tasks/`，长期规则见 [WORKFLOW](WORKFLOW.md)。

M3 保留分类正确率 >95% 的规划目标；M4 需新机安装到右键出结果的完整验证。证书、公证、brew tap 与 updater 尚未交付。

## 9. 质量与验证

金样体积比 / SSIM 相对基线劣化 >3% 失败；level≥5 的单代照片另须 SSIM≥0.95。动图、平涂图与二代 HEIC 由相对基线保护，度量原因见 [benchkit](../benchkit/README.md)。保留负优化拦截、undo 逐字节还原与输出尺寸守卫。

耗时、内存、CPU 受环境影响，只记录工具、负载和测量边界，不作为验收硬指标（用户确认，2026-11-18）。兼容性需抽验微信 / 浏览器 / Preview / QuickLook；人工交互不可由自动化推定通过。验证命令见 [AGENTS](../AGENTS.md)。
