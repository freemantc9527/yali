# DESIGN · Yali 产品与工程规格（定稿摘要）

> 本文件是新项目的设计权威。完整版历史与推导过程见私有档案；此处为自包含的结论态规格。

## 1. 产品定义

macOS 原生图片处理入口：**右键 0.5 秒发起、后台静默完成；需要精调时 GUI 里全是旋钮。两个世界，一个内核。**

| | 右键快捷操作世界 | GUI 世界 |
|---|---|---|
| 心智 | "别问我，直接压" | "我要控制每一个细节" |
| 参数来源 | 绑定的 Profile 预设档 | 实时编辑 Profile / 单文件覆盖 |
| 反馈 | 聚合通知（省了 X MB） | 队列 + 对比预览 + 统计 |
| 失败 | 自动转入 GUI 队列 | 就地重试/改参 |

设计原则：①Profile 是唯一事实源（所有入口消费同一份 JSON）②默认即最优（出厂档按实测标定）③渐进披露（右键永不询问）④全本地无遥测 ⑤arm64 原生 ⑥**无 Xcode 依赖**（见 TOOLCHAIN.md）。

系统基线：macOS 14+。

## 2. Profile Schema（核心数据结构）

存放 `~/Library/Application Support/app.yali/profiles/*.json`，camelCase 序列化，`deny_unknown_fields`：

```jsonc
{
  "name": "default",
  "pipeline": { "classify": true, "engine": { "jpeg": "mozjpeg" } },
  "resize":   { "maxWidth": null, "maxHeight": null, "keepRatio": true,
                "mode": "shrinkOnly",            // shrinkOnly | always | none
                "cropAnchor": null },            // center | smart
  "quality":  { "kind": "level",                 // level(1-10) | targetBytes | lossless
                "level": 4, "targetBytes": null, "minQualityFloor": 3 },
  "format":   { "output": "auto",                // auto|jpeg|png|webp|avif|heic|jxl
                "animated": "webp" },
  "metadata": { "exif": "privacy",               // keep | strip | privacy
                "icc": "srgb" },
  "output":   { "dir": "sibling",                // sibling | override | custom | downloads
                "suffixTemplate": "-{profile}",  // 支持 {profile}/{date}/{size}
                "largerPolicy": "keepOriginal",  // 负优化拦截
                "undoManifest": true },
  "ui":       { "notify": true, "sound": false, "openGUIOnFail": true }
}
```

出厂档：default（level 4，JPEG 类实测 -55~60%）、网页（webp q75 / avif 智能，-69~-78%）、归档（JXL 无损包裹，-17.2% 可逆）、朋友圈/微博（1080 缩边）、邮件瘦身（targetBytes 500KB 二分收敛）、透明截图（无损 WebP/oxipng，由分类器自动路由）。

同步机制：GUI 编辑 → 原子写 JSON → FSEvents 广播 → CLI/workflow 下次调用即生效，无 IPC。

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

## 4. 入口矩阵

| 通道 | 技术 | 阶段 |
|---|---|---|
| Finder 快速操作 | 用户级 `.workflow` ×2（压一下 / 转 JPG，Run Shell Script 调 `yali` CLI；按 UTI 逐动作声明，JPEG 文件不显示「转 JPG」） | **M0.5 主通道**（右键永不询问：选格式=预设动作，精调归 GUI） |
| 拖 Dock / 打开方式 | CFBundleDocumentTypes + open-file 事件 | M1 |
| 剪贴板监听 | arboard changeCount，菜单栏开关 | M2 |
| watch 文件夹 | notify crate（FSEvents），(path,mtime,size) 去重 | M2 |
| 菜单栏托盘拖入 | Tauri tray；file-drop 需 AppKit helper 补充 | M2 可选 |
| NSServices 子菜单 | objc2 胶水 + Info.plist | M2 评估 |
| CLI | `yali compress/convert/lossless/undo/profiles/doctor` | M0 |

「压一下」时序：workflow 收路径 → fork `yali compress --profile default --quiet` → tokio 队列 → 聚合通知 → 失败件写队列库待 GUI 承接。全程零弹窗。

## 5. GUI 规格（参数控制台）

- 布局：工具栏（添加/清空/再次压缩/统计/设置）→ 紧凑拖放条 → 任务列表（缩略图 + `766KB → 334KB (-56%)` + 状态 chip）→ 底部胶囊（Profile 选择/强度滑条/格式/本批共省）→ 右侧 Inspector（可折叠；折叠=查看器，展开=控制台）
- Inspector 六卡：尺寸 / 质量（level 滑条旁显预估体积）/ 格式（七选+动图策略+智能路由开关）/ 元数据（exif/icc 三态+去 GPS 快捷勾）/ 输出（dir/后缀模板/largerPolicy/undo）/ 管道专家（engine 覆盖/AVIF 速度档/并发/dither）
- 对比查看器：Space 唤起，before/after 滑杆 + SSIM 标注
- Profile 管理器：另存/导入导出 JSON；统计页：累计节省/格式分布/时间线
- 实现：React + TS + Vite + Tailwind；组件清单固定，禁止自造视觉体系
- 微交互：拖入悬停 / 行内进度环 / 完成 toast / ⚠ 负优化角标 / 行右键单文件覆盖 / ⌘↩ 批量应用 / ⌘Z 撤销本批

## 6. 架构

```
# 目标结构；标注「待建」的目录尚未存在，创建时机见 docs/STATE.md
yali/
├─ crates/yali-core          # Profile(serde) · Pipeline · EngineRegistry · Classifier
│                            # Metadata · UndoManifest · FileNamer · Metrics(rusqlite)
├─ crates/yali-cli           # clap 门面（安装二进制名 yali）
├─ crates/yali-integration   # 待建 · 平台差异 trait 收口（跨平台预埋）
├─ src-tauri                 # Tauri 壳：commands/tray/剪贴板/updater
├─ ui/                       # React+Vite（Tailwind 是否引入待 M1 任务卡评估）
├─ workflows/                # 待建 · .workflow 模板 + install-workflows.sh
├─ helpers/                  # 待建 · vision helper（CLT swiftc 编译的 Swift 小工具）
├─ benchkit/                 # 待建 · 合成金样 + bench.sh 回归
├─ schema/                   # ✓ 已建 · profile.schema.json（Profile 文档事实源）
└─ fixtures/                 # ✓ 已建 · 五档出厂 fixtures（default/web/archive/mail/screenshot）
```

- 引擎集成两步走：**Phase A 子进程**（Brewfile 钉版，tokio 进程池预热，GPL 边界干净）→ Phase B 库内链接（可选：imagequant-rs/oxipng/ravif/vips-sys；每步有 CLI 回退）
- 任务模型：并发 `min(active_cores-1, 4)`；状态机 `created→classifying→processing→done|failed|larger_output`；SQLite 持久化断点续
- 数据：`app.yali/metrics.sqlite`（统计）、`.yali-undo.json`（撤销清单，`yali undo` / GUI ⌘Z 消费）
- 更新：tauri-plugin-updater（minisign，与 Apple 证书解耦）

## 7. API 草图

```rust
#[async_trait]
pub trait Encoder { fn format(&self) -> Format;
    async fn encode(&self, img: &PixelImage, q: Quality) -> Result<Vec<u8>>; }

pub struct TaskQueue { /* tokio Semaphore + mpsc */ }
impl TaskQueue {
    pub async fn submit(&self, files: Vec<PathBuf>, profile: Profile) -> TaskSession;
    pub fn updates(&self) -> broadcast::Receiver<TaskEvent>; }

pub async fn compress(file: &Path, profile: &Profile) -> Result<CompressResult>;
// CompressResult { input, output, input_bytes, output_bytes, ssim, engine, elapsed, larger_than_input }
```

## 8. 里程碑与验收

| 里程碑 | 交付 | 验收（可执行） |
|---|---|---|
| M-1 整备 | 仓库/骨架/**CLT 终验** | ✅ 已过（2026-11-17） |
| M0 内核+CLI | schema+fixtures+core+CLI+benchkit | ①合成集回归达标 ②单图 p50<1.5s ③undo 100% 还原 ④`cargo test` 绿 |
| M0.5 右键闭环 | install-workflows + 聚合通知 | 真机右键→通知（人工验收） |
| M1 GUI | Tauri 壳+React 全界面+open-file | §5 覆盖；10 图批处理内存峰值<300MB |
| M2 入口全量 | 剪贴板/watch/tray | 20 图右键→通知 p50<8s |
| M3 智能 | 分类路由+标定表+Vision helper+全格式 | 截图路由正确率>95%；抠图<2s |
| M4 发布 | GitHub Release+brew tap+updater | 新机 brew install 到右键出结果<3min |

## 9. 测试与性能预算

- 金样回归：benchkit 合成集（照片/UI/动图/透明/HEIC），体积/SSIM/耗时三指标，劣化>3% 即红
- 质量门：level≥5 时**单代照片类样本** SSIM≥0.95（平涂图形/动图对边缘 ringing 过敏、HEIC 属二代生成，绝对值不适用，由基线漂移守护；口径实测记录见 benchkit/README.md）
- 负优化拦截零漏
- 预算：core 常驻<80MB；GUI<250MB；右键冷启动<400ms；100 图批处理不卡 Finder
- 兼容抽验：微信/QQ/钉钉发送、Safari/Chrome 显示、Preview/QuickLook
