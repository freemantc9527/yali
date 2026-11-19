# T-022 · 预设、字号、对比与统一数据目录

status: 已实施，待人工验收（2026-11-19）
goal: 按用户要求收敛四个内置预设，提供百分比字号，精简对比界面，将应用文件统一到 `~/.yali/`
manual-check: true

## 范围

- 内置预设按顺序为日常压缩、转 JPG、缩小尺寸、无损归档；缩小尺寸默认最长边 1024px，锁定宽高比、不放大小图。保留内部 `thumbnail` 标识。
- 应用设置使用齿轮；字号滑杆固定 80 / 100 / 120 / 150 / 180 / 200% 六档，默认 100%，保存后全局生效、重启保留。
- 对比分割仅保留画布上的拖动分割线，移除底部滑杆与说明，保留缩放、导航、SSIM。
- 960px 以下“处理设置”在任务主面板下方展开，与任务区共同滚动；宽屏保持右侧双栏。
- 预设、设置、撤销数据、缓存与临时数据统一使用数据根 `~/.yali/`，保留 `YALI_HOME` 测试重定向。首次访问复制旧应用缺失配置、不覆盖新目录；本机旧文件已按用户 11-19 要求确认并清理，见后节。
- 保持现有 UI 设计与输出图片目录策略，不操作用户 Services，不提交或创建分支。

## 验证

```text
cargo test -p yali-core -p yali-app：80 passed，0 failed（含 fixtures / doc suites）。
pnpm --dir ui test：9 files / 61 passed。
pnpm --dir ui build：TypeScript + Vite 成功。
bash scripts/build.sh：真实前端 / release / Yali.app 成功，14.78 MiB；10 类图片、10 个具体 UTI 校验通过。
cargo build -p yali-cli：成功，真实 CLI 验证使用重建后的二进制。
bash benchkit/bench.sh --profile default：6 张样图体积比 / SSIM 通过，基线未改。
git diff --check：通过。
```

金样首次在沙盒内被 sips HEIC 写出 Error 13 拦截，沙盒外完整复验通过。体积 / SSIM 与原基线一致；耗时只观察，photo/ui/palette/GIF/transparent/HEIC 三次中位数分别为 95/51/340/159/45/164ms。

隔离 HOME 的真实 CLI 验证：旧 v1 设置从 Library 复制到 `.yali/`，tray=false 保留、字号默认 100%、内存 schemaVersion=2；旧目录原件保留，仅列四个内置预设。1200×800 样图生成 320×213 缩略图；64×42 小图保持 64×42（通过转 WebP 排除同格式负优化导致无产物的干扰）。集中撤销记录写在 `.yali/data/undo/`，图片旁没有新增清单，`yali undo` 删除产物并保留原图。

新增回归覆盖：v1 设置读取升级 / 字号保存重读 / 越界保存拒绝；旧原版预设隐藏且修改档保留；真实 vipsthumbnail 大图缩小且不放大小图；集中撤销的目录隔离与旧旁置清单兼容。

浏览器渲染覆盖 1280×800、980×640、640×420、390×844，检查字号 100% / 150% / 200%、保存后全局字号、设置滚动区与底部保存入口。对比图使用浏览器生成的测试图片，验证画布无底部滑杆和说明区、方向键调整分割线、图像区延伸到窗口底部；不作为真实压缩质量或原生拖动手感验收。一次开发样图请求被 Vite allow list 拒绝，未放宽访问范围，之后改用 File / object URL。

正式生产预览复验：200% 字号下 390×844 与 640×420 均无横向溢出，百分比输入框随字号扩展、完整显示 200；底部保存入口始终位于视口内。生产预览 console 0 errors / 0 warnings，无失败资源请求。两张审美检查截图均压缩至 63KB 以下并在检查后清理。最终 `scripts/build.sh` 已包含该字号控件宽度修正。

KISS：沿用现有视图与 settings_save，不新增依赖。DRY：core 统一解析数据根；rem 统一字号比例。单一职责：预设播种、设置迁移、临时文件生命周期分别封装，图片原子发布仍使用输出目录的临时产物，避免跨磁盘 rename 破坏现有输出保证。

## 人工验收

- [ ] 原生应用四个内置预设、字号保存及重启恢复、分割线拖动、旧设置导入。

## 2026-11-19 补充调整与验证

- “缩略图”显示名改为“缩小尺寸”，默认 1024×1024 包围框，keepRatio=true、shrinkOnly；vipsthumbnail 不带 `!`，保持比例，不放大小图。只升级与上一版 320px 默认档完全相同的保存档，已修改参数保留。
- 字号滑杆用 0～5 的索引映射六个百分比，键盘每次换一档，Thumb 提供当前百分比 aria-valuetext；core 和浏览器桥接保存均拒绝非固定档。旧范围内连续值读取时归入最近档，例如 175→180，不改其他设置。
- macOS Wry 实现核验：incognito 使用 WKWebsiteDataStore::nonPersistentDataStore；窗口启用该配置，不继续积累 Library 网页缓存。应用设置仍经 core 保存到 `.yali/`。系统可重新维护文件选择器偏好，不能把系统偏好当业务配置使用。

```text
cargo test -p yali-core -p yali-app：82 passed，0 failed。
pnpm --dir ui test：9 files / 62 passed。
pnpm --dir ui build：TypeScript + Vite 成功。
cargo build -p yali-cli：成功。
bash scripts/build.sh：真实前端 / release / Yali.app 成功，14.78 MiB；10 类图片 / 10 个具体 UTI 校验通过。
真实 CLI compress --profile thumbnail：531019→19707 bytes，产物 1024×683。
git diff --check：通过。
```

新增测试覆盖未改动的 320px 档升级、保留自定义参数、连续字号归档与拒绝非法保存。真实 vips 回归检查大图变为 1024×683，再给更大包围框时仍不放大。浏览器实际键盘 Home→五次 ArrowRight 依次显示 80/100/120/150/180/200%，保存 200% 后根字号 32px；390×844、640×420 无横向溢出，滑杆在设置滚动区、保存按钮固定可达；恢复 100% 后 980×640 审美检查。console 0 errors / 0 warnings，资源无失败。

本机清理先逐文件比较旧 Application Support 与 `.yali/`，确认六份预设一致（`.seed` 签名升级除外）；废弃预设只在语义等于原版时列入删除。按项目规则取得执行确认后，移除：

- `~/Library/Application Support/app.yali/`
- `~/Library/Caches/app.yali/`
- `~/Library/WebKit/app.yali/`
- `~/Library/Preferences/app.yali.plist`（旧文件选择器偏好）
- `~/.yali/profiles/{web,mail,screenshot}.json`

共 50 个文件 / 11,641,819 bytes；清理脚本执行后逐路径确认不存在，`.yali/profiles/` 仅余 archive/default/jpg/thumbnail，thumbnail 为 1024px、keepRatio=true、shrinkOnly。没有删除图片、用户修改预设、Services 或开发材料；不扫描任意图片目录删除旁置撤销记录。用户原生亲验项继续保留待验。

### 字号刻度精简（2026-11-19）

移除滑杆上方重复的当前百分比及未使用的样式，下方六个刻度均增加 `%`；保留 Thumb 的当前百分比辅助技术信息。KISS：只调整展示，不新增状态或依赖。

`pnpm --dir ui test`：62 passed；`bash scripts/build.sh`：UI / release / Yali.app 构建成功，14.78 MiB，10 类图片 / 10 个具体 UTI 校验通过。生产浏览器 390×844、100% 与 200% 字号：无上方数值，六个百分比刻度完整显示、互不重叠、无横向溢出；键盘切档与保存正常。`git diff --check` 通过。原生亲验状态不变。

### 窄窗口设置位置（2026-11-19）

移除窄屏反转顺序的 CSS，沿用 DOM 中主面板在前、设置在后的顺序；分隔线改为设置顶部边框。任务主面板禁止 flex 压缩，长列表按内容撑开后与设置共同滚动。KISS：只调整响应式布局，继续复用同一表单与状态。

`pnpm --dir ui test`：9 files / 62 passed；`bash scripts/build.sh`（含 `pnpm --dir ui build`）：UI / release / Yali.app 成功，14.78 MiB，10 类图片 / 10 个具体 UTI 校验通过。生产浏览器空状态覆盖 960/959/800/640/390px，12 项任务覆盖 390/640/959/960px：窄屏设置位于主面板及最后任务行之后、滚动可达设置末尾、底栏位于视口内，无横向溢出；展开/收起及跨断点状态保留正常。console 0 errors / 0 warnings，无失败资源请求。`git diff --check` 通过；原生亲验仍待人工验收。

### 提交前审查与迁移中断修复（2026-11-19）

对全部未提交变更做深度审查，安全与架构复核发现一项中等严重程度问题：旧配置直接复制到最终路径时，中断会留下残缺文件，重试因目标存在而跳过，导致设置回落默认。经用户确认修复后，改为同分区独立临时目录完整复制并同步，再通过 hard_link 原子发布且不覆盖已有目标；复用 TempDir 清理正常失败路径，不新增依赖。安全、架构及最终四角度对抗复核未发现其他新增问题。数据根和迁移约束同步到 AGENTS 与 DESIGN。

- 回归测试 `interrupted_import_does_not_publish_partial_files_and_can_retry` 在旧实现上因“失败复制不得留下最终文件”失败；修复后 `cargo test -p yali-core -p yali-app`：83 passed。
- `pnpm --dir ui test`：9 files / 62 passed；`cargo build -p yali-cli` 成功。
- 重建后的真实 CLI 在隔离 HOME 中注入 128-byte 文件写入上限，旧实现中断留下 128-byte 文件、重试将 tray=false 回落为 true；修复后中断不发布最终文件，重试 exit 0，tray=false 保留，源目标逐字节相同。8 进程并发迁移全部 exit 0，原配置完整保留；CLI help、四档列表及 1024px 缩小尺寸配置通过。
- `bash benchkit/bench.sh --profile default`：六张金样体积比 / SSIM 均通过。沙盒内 HEIC 的 sips 写出被 Error 13 拦截，取得沙盒外执行授权后完整复验通过；耗时仅观察，未改基线。
- `bash scripts/build.sh`（含 UI build）：release / Yali.app 成功，14.78 MiB；10 类图片 / 10 个具体 UTI 校验通过。`git diff --check` 通过。

本轮提交范围为 T-022 全部变更；未推送或发布，原生交互仍待人工验收。回退代码可 revert 本轮提交；旧配置迁移只复制、不删除原件，已迁移到 `.yali/` 的配置不随代码回退删除。
