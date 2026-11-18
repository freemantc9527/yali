# STATE · 项目状态接力（跨 session 冷启动先读本文件）

> 更新规则：每张任务卡完成时更新；本文件是 Agent 的「记忆外置盘」，只保留**当前真相**，
> 历史细节进 `docs/tasks/` 卡片与 git log。最后更新：2026-11-18（T-020：品牌图标与菜单栏模板图人工核验通过）。

## 0. 冷启动清单（新会话第一件事）

```bash
cd /Users/taocheng/career/yali
git pull origin main && git log --oneline -3
cargo test                 # 期望：61 passed（core 53 + cli 3 + app 3 + fixtures 2），零警告
pnpm --dir ui test         # 期望：49 passed
./benchkit/bench.sh        # 期望：三指标均在阈值内（基线按档位分文件）
env -u CI cargo tauri build --bundles app && ./scripts/bundle-check.sh   # 整包 + UTI 校验
```

阅读顺序：本文件 → `docs/DESIGN.md`（设计权威）→ `docs/WORKFLOW.md`（任务卡制度）→
`docs/tasks/README.md`（卡索引）→ 要动质量再读 `benchkit/README.md`（度量口径与偏离）。

## 1. 里程碑状态

| 里程碑 | 状态 | 说明 |
|---|---|---|
| M-1 整备 | ✅ | 仓库/骨架/**CLT-only 终验**（ADR-002 假设成立，未触发 ADR-003） |
| M0 内核+CLI | ✅ | 出口四条全过（见下表） |
| M0.5 右键闭环 | 🔶 | 自动部分完成并实测；**Finder 菜单人工复核待办** |
| M1 GUI | 🔶 | 五卡落地 + open-file（T-013）；**真机拖拽/滑杆/内存峰值/「打开方式」待复核** |
| M2 入口全量 | 🔶 | T-013…T-017 落地；余下 队列库/统计页 2 卡 |

### M0 出口核验（2026-11-17 收口，全部通过）

- [x] 合成集回归达标：bench default/web/jpg 全绿；level≥5 单代照片 SSIM≥0.95（实测 0.9657）
- [x] 单图 p50 < 1.5s：实测 **p50 = 119ms**（预算 8%）
- [x] undo 100% 还原：sha256 逐字节一致 + 三场景单测
- [x] `cargo test` 全绿 + `pnpm --dir ui test` 全绿，零警告

## 2. 已完成任务卡（细节与验收在 `docs/tasks/`）

| 卡 | 内容 | 关键成果 |
|---|---|---|
| T-002 | Profile v1 Schema + Store + 出厂 fixtures | 六段结构、camelCase+deny_unknown_fields、原子写、YALI_HOME |
| T-003 | EngineRegistry + CLI 真实现 | 11 个子进程引擎、level→参数映射、targetBytes 二分、负优化拦截、undo/doctor |
| T-004 | benchkit 合成金样 + 三指标回归 | 确定性样张、劣化>3% 退出码非零、基线按档位分文件 |
| T-005 | UI 接线（脚手架→可用） | Tauri commands + App/ProfileForm/TaskList/DropZone + bridge 浏览器降级 |
| T-006 | M0.5 右键闭环 | `.workflow` 模板 + 安装脚本 + CLI `--notify` 聚合通知 |
| T-007 | Profile 编辑与保存 | CLI `profiles save` + `profiles_save` + 可编辑表单 |
| T-008 | 输出目录策略 + schema v1.1 | sibling/downloads/custom/override、`dirPath`、`{size}` 两段式命名 |
| T-009 | 并发队列 + 进度事件 | Semaphore(min(cores-1,4))、`yali://progress`、UI 纯 reducer |
| T-010 | 对比查看器 | `metrics.rs`（ffmpeg SSIM 口径）+ asset protocol + 滑杆 Viewer |
| T-011 | Profile 管理器 | 另存为 / 导出剪贴板 / 粘贴导入 |
| T-012 | 右键动作取舍 | 两条动作（压一下 / 转 JPG）、逐动作 UTI 声明、出厂档 `jpg`、`.seed` 补种 |
| T-013 | 真 .app + open-file 入队 | bundle.active + 精确 UTI（Info.plist 覆盖）+ `RunEvent::Opened` + 冷启动缓冲 + capabilities（事件监听修复） |
| T-014 | 应用设置 settings.json | 入口开关/ watch 目录/防抖，语法坏→备份回落、字段错→报错，CLI `settings show/path/reset` |
| T-015 | 菜单栏托盘 + 关窗常驻 | `tray-icon` feature、`RuntimeSettings` 即时生效、关闭按钮 → 窗口 0 进程仍在（自动验证） |
| T-016 | 剪贴板监听（复制即压） | NSPasteboard 主线程轮询 + FNV 去重 + TIFF 回落 sips；产物落「下载」；实测 -84% 尺寸不变 |
| T-017 | watch 文件夹 | notify/FSEvents + settle 防抖 + (mtime,size) 去重 + 产物 ignored；实测无自激、覆盖重压 |
| T-020 | 品牌图标与菜单栏模板图 | 1024px 透明吉祥物、苔绿色圆角应用图标集、单色模板图；人工核验通过 |

## 3. 待人工复核（Agent 无法自验，`manual-check`）

- [x] Finder 右键「快速操作」里有且仅有两条鸭梨；**JPEG 文件上看不到「转 JPG」**（用户核验 2026-11-17）
- [x] 多选只弹**一条**聚合通知，文案含「N 张省了 X」（用户核验 2026-11-17）
- [x] Finder「打开方式」里出现 Yali；Dock 拖到图标上能入队（用户核验 2026-11-17）
- [x] 品牌应用图标与放大后的单色菜单栏图标显示、尺寸和辨识度（用户核验 2026-11-18，T-020）
- [ ] 真机 `cargo tauri dev`：拖入 → 任务行 → 结果 → 对比滑杆手感
- [ ] 10 图批处理内存峰值 < 300MB（Instruments 抽验，DESIGN §8）
- [ ] 产物在微信 / 浏览器 / Preview 显示抽验（每里程碑一次）

复核通过后：勾掉本节并把 M0.5 / M1 标 ✅。

## 4. 下一步（建议顺序）

1. 先做完第 3 节的人工复核（不然后面都在没验证的地上盖楼）
2. **M2 余下卡片**：T-018 失败件队列库（SQLite 断点续）→ T-019 统计页（metrics.sqlite）
   - 新依赖 `rusqlite` 待批（T-013…T-017 已用：`tray-icon` feature、`objc2` 系、`notify`）
3. M2 出口验收：20 图右键 → 通知 p50 < 8s（CLI 可先跑基线）
   - 托盘菜单展开/唤回窗口属人工（AX 点 status item 拿不到菜单层级）
4. 技术债（有卡再做）：`helpers/` vision helper、`crates/yali-integration`、
   M3 标定表 `calibration.json`（替换 level→参数 v1 公式）

## 5. 关键决策（摘要，全文在 `docs/decisions/`）

- 技术栈：Tauri 2 + Rust + React；**硬约束无 Xcode**（ADR-002，ADR-003 备而未用）
- 前端包管理器：**pnpm**，命令一律 `pnpm --dir ui …`（ADR-004）；锁文件入库
- 右键主通道：用户级 `.workflow`（免 Xcode 免签名）；NSServices 为 M2 可选
- **右键永不询问**：选格式 = 预设动作（当前两条），精调归 GUI/CLI；快速操作里鸭梨 ≤3 项
- 命名：鸭梨/Yali，产出物名称见 `docs/NAMING.md`；品牌语境 Yali，技术语境小写 yali
- 分发：本地工具定位，Developer ID 证书延后；brew tap 源码构建为主渠道
- 平台：macOS 先行，Linux→Windows 备忘见 `docs/CROSSPLATFORM.md`

## 6. 坑账本（随踩随记，二次复发升级为 AGENTS 硬性规则）

**流程类**
- 报「全绿」前必须逐行看到 `test result: ok`；聚合计数管道会静默失败（本仓踩过两次）
- 改内核后 **debug 与 release 都要重建**：`bench.sh` 默认用 debug 二进制，会拿旧结果骗人

**引擎与图像**
- brew 的 `cjpeg` 是 libjpeg-turbo 李鬼；mozjpeg 必须走 keg 路径（有反李鬼单测）
- **`vips flatten` 对无 alpha 的 PNG（尤其 Type:Palette）会输出 graya 灰白图**（实测 PSNR 0.6
  报废）：PNG→JPEG 必须先读 IHDR color type（调色板再查 tRNS）确认真有透明才 flatten，
  且 flatten 产物 color type 必须为 2，否则回落 sips
- **cjpeg 与 libvips 都把 RGBA PNG 的 alpha 合成到黑底**（PSNR 2.2dB）：有透明才压平到白底
- **pngquant 对已量化的调色板 PNG 退出码 99**：调色板 PNG→PNG 强制 oxipng 无损；
  有损档位落到 oxipng 用 `-o 1`（353ms/800966B，`-o 4` 多花 750ms 只多省 690B）
- **vipsthumbnail 不带 `--size` 默认输出 128×128**（GIF→JPG 被缩成 128×96）：兜底转换用
  `vips copy`；预缩放用 vipsthumbnail 但必须显式传 `--size`
- **vipsthumbnail 不认 dotfile 输出路径**（图片写进 stdout，退出码还是 0）：中转文件用非隐藏名
- **libvips 的 heif loader 只返回内嵌缩略图**（1920×1280 → 128×85）：HEIC/AVIF/JXL 容器解码走 sips
- libvips 冷启动 ~5s → 进程池预热是设计要求不是优化项
- 管道有**输出尺寸守卫**（容器/vips/预缩放路径生效）：编码器悄悄降采样会拒绝落盘

**Finder 右键（.workflow）**
- 只装 bundle + `flush pbs` 不上菜单：必须写 `pbs` 的 `NSServicesStatus` + `FinderOrdering`
- **`FinderOrdering` 的值必须是 int**：`defaults write ... 7` 写成字符串会让 Finder 解析整表
  失败并**吞掉「快速操作」整栏**；改用 `defaults export/import` + plistlib
- **`NSSendFileTypes` 必须列具体 UTI**：只写抽象 `public.image` 在正常目录不匹配（废纸篓宽松）
- Services 的 PATH 只有 `/usr/bin:/bin`：引擎解析必须自带 Homebrew 目录兜底
- **`brew --prefix` 在 HOME 陌生时会先拉 API 数据**（网络不畅可挂数分钟）：引擎解析不调 brew，
  直接探 `/opt/homebrew/opt/<pkg>` 与 `/usr/local/opt/<pkg>`
- plasma/噪声类合成图必须 `-seed` 固定随机，否则样张每次不同、基线漂移无意义

**Tauri / 构建**
- **没有 `src-tauri/capabilities/default.json`（`core:default`）时前端 `listen()` 被 ACL 拒绝**：
  报错只在 WebView 控制台，事件静默失效。T-013 实测——T-009 的 `yali://progress` 此前只在浏览器降级
  路径验证过，真机上一直是哑的
- `beforeDevCommand` / `beforeBuildCommand` 的 **cwd 是 `src-tauri`**：必须写 `pnpm --dir ../ui …`
  （写 `--dir ui` 直接 No such file or directory；`cargo tauri dev` 同样中招）
- **环境里有 `CI=1` 时 cargo-tauri 启动即报 `invalid value '1' for '--ci'`**（clap 把 CI 当布尔）：
  整包命令固定 `env -u CI cargo tauri build --bundles app`
- `bundle.macOS.infoPlist` 路径按 **src-tauri** 解析（写 `Info.plist`）；顶层键**整键覆盖** bundler 结果
- tauri-bundler 对不认识的扩展名（avif/jxl）会兜底塞抽象 `public.image` → 会把「打开方式」
  扩大到所有图片格式；用 `src-tauri/Info.plist` 覆盖，`scripts/bundle-check.sh` 守漂移
- 托盘验收技巧：`System Events` 里第三方状态项是 `menu bar 2` 的 `status menu`；
  关窗常驻可用 AX 点 `AXCloseButton` + `pgrep` 自动验证（T-015 已用）
- `tauri::generate_context!` **编译期**要求 `src-tauri/icons/icon.png` 存在
- 图标必须是 RGBA8：gray+alpha 会让应用启动 panic「pixel count mismatch」
- **debug 不校验 `frontendDist`、release 校验**（缺失 panic）：`src-tauri/build.rs` 已兜占位页；
  验证仓库可复现性必须跑 `--release`
- 迁移路径后 `target/` 缓存带旧绝对路径，指纹脏：换路径后 `cargo clean` 重建一次
- gh 2.104 无 `-g` 短参；`gh repo create` 远端新建用 `--add-readme --license --clone`
