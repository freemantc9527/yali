# STATE · 项目状态接力（跨 session 冷启动先读本文件）

> 更新规则：每张任务卡完成时更新；本文件是 Agent 的"记忆外置盘"。最后更新：2026-11-17（T-009 完成 · M1 进行中）。

## 当前阶段：M0.5 右键闭环（自动部分完成，人工项待复核）→ M1 GUI

- [x] GitHub 仓库 `freemantc9527/yali`（public，Apache-2.0）
- [x] README / AGENTS.md / .gitignore / Brewfile
- [x] 提交身份：noreply 邮箱 + gh 凭证助手
- [x] Cargo workspace 骨架（yali-core / yali-cli）
- [x] **CLT-only 终验通过（2026-11-17 10:27）**：`cargo build -p yali-app` Finished；`cargo test` 3 passed——ADR-002 核心假设成立，无需触发 ADR-003 降级
- [x] src-tauri 占位图标（generate_context! 编译期依赖 icons/icon.png）
- [x] **ADR 入库（2026-11-17）**：`docs/decisions/` ADR-001（废弃）/ ADR-002（接受）/ ADR-003（兜底备用）/ ADR-004（pnpm）
- [x] **ui/ 最小 Vite 脚手架（T-005 部分完成）**：Vite 8 + React 19 + TS + vitest；`pnpm --dir ui test` 4 passed；`pnpm --dir ui build` → `ui/dist`
- [x] **release 构建坑修复**：`frontendDist` 缺失会让非 dev 构建 panic → `src-tauri/build.rs` 在 codegen 前兜占位页
- [x] docs/ 自包含规格迁移（DESIGN/BENCHMARKS/TOOLCHAIN/NAMING/WORKFLOW/CROSSPLATFORM）
- [x] **T-002 Profile Schema + Store + 五档 fixtures（2026-11-17）**：`schema/profile.schema.json`（文档事实源）+ `fixtures/{default,web,archive,mail,screenshot}.json` + `crates/yali-core/src/{profile,store}.rs`（六段 Schema / YALI_HOME 重定向 / 原子写 / 出厂播种）；`cargo test -p yali-core profile` 15+2 绿，零警告；CLI `profiles list/show` 从磁盘读取，`show default` 与 fixture 逐字节一致
- [x] **T-003 EngineRegistry + CLI 真实现（2026-11-17）**：tokio 引回（子进程池 + 并发 min(cores-1,4)）；Encoder trait + 11 引擎注册（mozjpeg keg 路径反李鬼 / pngquant / oxipng / gifsicle / gif2webp / cwebp / avifenc / cjxl / svgo / sips / vips 兜底）；魔数嗅探；targetBytes ≤5 轮 level 二分；负优化拦截；UndoManifest；doctor/compress/convert/lossless/undo 全通
- [x] **T-004 benchkit 合成金样 + 三指标回归（2026-11-17）**：`generate.sh` 确定性生成五类样张（seed 固定，两次生成哈希一致）；`bench.sh` 输出 体积比/SSIM/耗时中位数，与 `baseline.json` 比对（劣化>3% 退出码非零，实测触发）；default/web 双档全绿
- [x] **T-005 UI 接线（2026-11-17）**：Tauri commands（profiles_list/profiles_show/compress，壳层零业务逻辑）+ App/ProfileForm/TaskList/DropZone + bridge 浏览器降级；`pnpm --dir ui test` 19 绿；command 层真压缩冒烟（benchkit 样张）通过；应用启动冒烟通过（图标修成 RGBA8）；浏览器端到端验证：切档表单联动、拖入→`748.0 KB → 326.2 KB (-56.4%)` 行渲染
- [x] **T-006 M0.5 右键闭环（2026-11-17）**：`workflows/`（模板 + install-workflows.sh）装到 `~/Library/Services/鸭梨 · 压一下.workflow`，pbs 注册可见；CLI 新增 `compress --notify` 聚合通知（多选一条，文案三态单测）；引擎解析加 Homebrew 绝对路径兜底（Services PATH 只有 /usr/bin:/bin）；`automator -i` 端到端跑通产出 `-{profile}` 文件
- [ ] **人工复核（Agent 无法自验）**：Finder 右键菜单是否出现「鸭梨 · 压一下」、多选只弹一条通知
- [x] **T-007 Profile 编辑与保存（2026-11-17）**：CLI `profiles save`（stdin/--file，落盘前 validate）+ Tauri `profiles_save`（command 冒烟含非法值拒绝）+ UI 可编辑表单（profile-edit 纯函数层，校验以内核为准）；浏览器实测：改 level→保存→「✓ 已保存」、清空后缀→「✗ 后缀模板不能为空」
- [x] **T-008 输出目录策略 + schema v1.1（2026-11-17）**：`output.dirPath` 入 schema/fixtures；sibling/downloads/custom/override 四策略实测通过（downloads 重定向 HOME、override 读 `YALI_OUTPUT_DIR` 且缺失回落 sibling）；`{size}` 编码后渲染（photo-customout-48K.jpg）；undo 在 custom 目录按清单还原；engine 解析去掉 `brew --prefix` 子进程（HOME 陌生时会挂）改探 opt/<pkg>
- [x] **T-009 任务队列与进度事件（2026-11-17）**：Tauri compress 改并发（Semaphore，min(cores-1,4)）并逐文件发 `yali://progress`（started/done/failed + index/total）；UI 用纯 reducer（applyProgress/reconcile，乱序安全、事件丢失兜底）；浏览器实测 3 文件逐行更新；cargo 42 + ui 37 全绿
- [x] `docs/tasks/` 任务卡目录（T-002 起）
- [x] benchkit/ 合成金样素材生成脚本（T-004）

## 关键决策记录（摘要，全文在 docs/decisions/）
- 技术栈：Tauri 2 + Rust + React；**硬约束无 Xcode**（ADR-002）
- 前端包管理器：**pnpm**，命令一律 `pnpm --dir ui …`（ADR-004）；锁文件入库，`node_modules/` 不入库
- 右键主通道：用户级 `.workflow`（免 Xcode 免签名）；NSServices 为 M2 可选
- 命名：鸭梨/Yali，全部产出物名称见 docs/NAMING.md
- 分发：本地工具定位，证书延后；brew tap 源码构建为主渠道
- 平台：macOS 先行，Linux→Windows 备忘见 docs/CROSSPLATFORM.md

## M0 出口条件核验（2026-11-17 收口，全部通过）

- [x] 合成集回归达标：`benchkit/bench.sh` default/web 双档全绿；level 5 照片类 SSIM 0.9657 ≥ 0.95（DESIGN §9 口径）
- [x] 单图 p50 < 1.5s：benchkit photo 样张 5 次实测 **p50 = 119ms**（预算的 8%）
- [x] undo 100% 还原：CLI 端到端 sha256 逐字节一致 + 单测覆盖三种场景
- [x] `cargo test` 全绿（core 32 + fixtures 2 + app command 1）且 `pnpm --dir ui test` 全绿（19）；零警告
- [x] STATE 更新为 M0 完成，`docs/plans/PLAN-M0.md` 已移除

## 下一步（按序）
> 任务卡在 `docs/tasks/`（索引见 `docs/tasks/README.md`）。

1. **M1 进行中**：计划见 `docs/plans/PLAN-M1.md`；T-007 ✅ T-008 ✅ T-009 ✅ → 下一张 T-010（对比查看器）/ T-011（Profile 管理器）
2. 待人工复核项：Finder 右键菜单出现与通知文案（T-006 manual-check）

## 坑账本
- `tauri::generate_context!` **编译期**要求 `src-tauri/icons/icon.png` 存在（即使 bundle.active=false）
- **debug 与 release 对 `frontendDist` 的校验不同**：debug 因 `dev_url` 有值直接短路，release/codegen 会 panic「frontendDist doesn't exist」。`ui/dist` 是构建产物不入库 → 已由 `src-tauri/build.rs` 兜占位页；验证仓库可复现性时必须跑 `--release`
- gh 2.104 无 `-g` 短参；`gh repo create` 远端新建用 `--add-readme --license --clone` 组合
- 迁移后 `target/` 缓存带着旧绝对路径，指纹脏；换路径后建议 `cargo clean` 重建一次
- **vipsthumbnail 不认 dotfile 输出路径**（`-o .foo.png` 会把图片写进 stdout，退出码还是 0）——预缩放中转必须用非隐藏文件名，且跑完校验文件存在
- **`brew --prefix` 在 HOME 陌生时会先拉 API 数据**（网络不畅可挂数分钟）——引擎解析不调 brew，直接探 `/opt/homebrew/opt/<pkg>` 与 `/usr/local/opt/<pkg>`
- **Services（右键 .workflow）的 PATH 只有 `/usr/bin:/bin`**：引擎解析必须自带 Homebrew 兜底，brew 命令本身也要绝对路径（/opt/homebrew/bin/brew）
- **libvips 的 heif loader 会返回内嵌缩略图**（1920×1280 的 HEIC 只解出 128×85）——HEIC/AVIF/JXL 容器解码走 sips，管道有输出尺寸守卫兜底（benchkit 首跑抓到）
- **本机 ImageMagick 7.1.2-36 的 `compare -metric SSIM` 语义损坏**（同像素输出 0 而非 1）——benchkit 的 SSIM 用 ffmpeg ssim 滤镜
- plasma/噪声类合成图必须 `-seed` 固定随机，否则每次生成内容不同、基线漂移无意义
