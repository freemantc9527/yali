# DESIGN · 产品与工程规格

当前进度见 [STATE](STATE.md)，界面行为与断点以 [UI-REDESIGN](UI-REDESIGN.md) 为准。本文区分已实现能力与后续方向。

## 1. 产品定义

Yali 是 macOS 14+ 本地图片压缩与转换工具：Finder 快速操作静默使用已保存预设，GUI 支持本批调参与结果对比，CLI 提供可脚本化入口。所有入口共用 Rust 内核与 Profile JSON，无账户、云端或遥测。

原则：Profile 统一参数；右键永不询问、精调归 GUI / CLI；保留原图；原生 arm64；开发仅依赖 CLT，无 Xcode。失败件持久化转入 GUI 属 T-018，当前不承诺。

## 2. 数据契约

Profile 的完整字段与默认档见 [Schema](../schema/profile.schema.json) 和 [fixtures](../fixtures/)；存储于 `~/Library/Application Support/app.yali/profiles/*.json`，camelCase、拒绝未知字段、原子写。`YALI_HOME` 可重定向数据根。

六档：default（日常压缩）、web（网页图片）、jpg（转 JPG）、mail（目标大小）、archive（JPEG 的 JXL 无损包裹）、screenshot（截图无损）。Finder 只绑定 default / jpg。GUI 本批草稿不自动写盘，明确保存后才影响后续入口；CLI / workflow 下次调用读取新文件，不依赖预设广播同步。

Schema 字段不等于已实现能力：metadata.exif/icc、pipeline.classify、resize.cropAnchor、output.largerPolicy 的完整执行效果尚未验证；GUI 保留值，不增加保证生效的控件。内容分类与智能裁剪属 M3。

应用配置独立存于同根 `settings.json`：schemaVersion=1；默认 clipboard=false、minBytes=65536；watch=false、dirs=[]、settleMs=800；tray=true、notify=true。缺文件 / 缺字段用默认；语法坏先备份 `.bad-<毫秒>` 再回落；未知字段、类型错误或未来版本报错且不覆盖。GUI 显式保存后应用，详细契约与历史证据见 [T-014](tasks/T-014-settings-store.md)。

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

响应式工作台、预设管理、设置与独立对比视图已由 [T-021](tasks/T-021-responsive-workbench.md) 实施，待原生人工验收。用户已确认效果图、shadcn/ui（Radix）、Tailwind CSS 4 与 Lucide，保留 React / TypeScript / Vite / Tauri。

完整行为、状态机、接口、视觉与响应式规则只维护在 [UI-REDESIGN](UI-REDESIGN.md)，不在本文重复。

## 6. 当前架构

| 目录 | 职责 |
|---|---|
| `crates/yali-core` | Profile / Settings Store、Pipeline、EngineRegistry、命名、undo、SSIM |
| `crates/yali-cli` | clap 命令门面，安装命令名 yali |
| `src-tauri` | commands、batch、preview、open-file、托盘、剪贴板、watch；capabilities 与精确 Info.plist |
| `ui` | React / Vite / shadcn / Tailwind，bridge 集中原生调用，reducer 管理会话批次 |
| `scripts` / `workflows` | dev / build / bundle-check；两条 Finder 动作的模板与安装 |
| `benchkit` / `schema` / `fixtures` | 合成质量回归、数据契约、六档出厂预设 |

引擎当前通过子进程调用，GUI 批内并发 `min(cores-1,4)`。预览只生成应用缓存副本；原图与真实结果路径不被预览替换。

尚未创建：`helpers/`、`yali-integration`、`yali-vision`。SQLite 队列 / metrics、GUI 撤销、updater、库内引擎链接均为后续方向，不是当前实现。

## 7. 接口定位

真实类型与函数以代码为准：core 的 profile / pipeline / engines / undo / metrics；CLI 的 main；Tauri commands 与 batch / preview；前端 bridge。定位前按 [AGENTS](../AGENTS.md) 使用 CodeGraph。GUI 新 command 合同见 UI 规格，不维护容易漂移的伪代码 API。

## 8. 里程碑

M-1 工具链 → M0 内核 / CLI → M0.5 Finder → M1 GUI → M2 入口 / 队列 / 统计 → M3 智能与标定 → M4 发布。当前完成度与人工待验统一见 [STATE](STATE.md)，各卡证据见[任务索引](tasks/README.md)。

M3 保留分类正确率 >95% 的规划目标；M4 需新机安装到右键出结果的完整验证。证书、公证、brew tap 与 updater 尚未交付。

## 9. 质量与验证

金样体积比 / SSIM 相对基线劣化 >3% 失败；level≥5 的单代照片另须 SSIM≥0.95。动图、平涂图与二代 HEIC 由相对基线保护，度量原因见 [benchkit](../benchkit/README.md)。保留负优化拦截、undo 逐字节还原与输出尺寸守卫。

耗时、内存、CPU 受环境影响，只记录工具、负载和测量边界，不作为验收硬指标（用户确认，2026-11-18）。兼容性需抽验微信 / 浏览器 / Preview / QuickLook；人工交互不可由自动化推定通过。验证命令见 [AGENTS](../AGENTS.md)。
