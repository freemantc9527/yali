# T-021 · shadcn/ui 响应式图片工作台

status: 已实施，待人工验收（2026-11-18）
goal: 添加、配置、批量处理、对比和查看结果的响应式工作台
context: [UI 完整规格](../UI-REDESIGN.md)；[ADR-005](../decisions/ADR-005-shadcn-responsive-ui.md)；[设计原图](../../design/ui-concept-01/yali-workbench.png)
manual-check: true

## 交付范围

已用 shadcn CLI 4.23.0 引入 new-york Radix 组件，采用 Tailwind 4.3.5、Lucide 与 Sonner、响应式工作台、本批草稿 / 预设分离、设置与独立对比、dialog / opener、inspect 与限流预览缓存、compress_batch 草稿快照及逐项结果、单 GUI batch 调度与 default 系统投递队列。完整断点、状态和错误规则只维护在 UI 规格。

保留 React / TS / Vite 版本范围、Safari 17 和既有 core / CLI 算法及 Schema；不做持久化失败队列、统计、取消、GUI 撤销或智能能力。KISS：reducer + props；DRY：新旧 command 共用执行器，体积展示从字节统一计算；单一职责：批次、预览与视图各自分离。

## 验证证据（2026-11-18）

### 初次实施的自动验证

```text
基线：ui 49 passed；yali-app 18 passed。
初次实施 pnpm --dir ui test：9 files / 59 tests passed（审查修复后为 61 项，见末节）。
pnpm --dir ui build：TypeScript + Vite 成功，Safari 17；无构建告警。
  设置/对比按需拆分，主 JS 约 492 kB（gzip 153 kB），Viewer 6.17 kB，SettingsView 11.50 kB。
cargo test -p yali-app：20 passed；0 failed；另 main/doc tests 各 0。
env -u CI cargo tauri build --bundles app：成功，Yali.app 14.75 MiB。
./scripts/bundle-check.sh：10 类图片、10 个具体 UTI，通过，无 abstract 类型。
git diff --check：通过。
```

未动 core，无需因本卡重跑 core 测试。全仓 `cargo fmt --all -- --check` 检出既有格式差异；仅对本卡 Rust 文件执行 rustfmt，未批量格式化其他模块。

### 金样回归：体积 / SSIM 无劣化，耗时仅观察

先构建 debug CLI，保持原基线。沙盒内 HEIC sips 写出报 Error 13，沙盒外可完成编码。

| 样张 | 输入 bytes | 输出 bytes | SSIM | 3 次耗时中位 ms | 7 次复核中位 ms |
|---|---:|---:|---:|---:|---:|
| photo.jpg | 531019 | 49178 | 0.964656 | 95 | 94 |
| ui.png | 34757 | 4696 | 0.999801 | 50 | 60 |
| palette.png | 921669 | 800966 | 1.000000 | 342 | 349 |
| anim.gif | 966363 | 661199 | 0.822700 | 160 | 178 |
| transparent.png | 32978 | 5276 | 0.932026 | 46 | 65 |
| photo_heic.heic | 263571 | 257639 | 0.989040 | 171 | 239 |

`benchkit/bench.sh --profile default`：退出 1，HEIC 耗时 +4.9%。
`benchkit/bench.sh --profile default --runs 7`：退出 1，ui.png +15.4%、GIF +11.9%、transparent.png +30.0%、HEIC +46.6%。上述退出码来自调整前的脚本，**体积 / SSIM 均未劣化**。7 次复核期间与整包构建有重叠，存在竞争负载，不能据此归因代码回归。用户于 2026-11-18 确认耗时与系统资源仅作观察；脚本现仅以体积比 / SSIM 判定回归，不再因耗时失败。保留历史输出，不改基线。

### 实际渲染与交互

- 检查 1280×800、1100×700、1099×700、980×640、960×640、959×640、800×600、799×600、640×420、639×600、390×844；页面无横向溢出，底栏和主按钮在视口内，始终一份参数表单。窄屏初始收起、跨断点保留用户开关状态。
- 浅色 980/1280、最小窗 640、窄屏 390 截图对照；深色通过应用已有 dark media 中的同一组 tokens 做浏览器渲染验证。窄屏工具栏触控实测 44px，滑杆/对话框关闭有中文 accessible name。正文与 Inspector 对比度 4.99:1，主按钮 5.28:1，控件边界 3.70:1。
- 浏览器 File/object URL 实图验证空、pending、处理中继续添加、3 成功/1 未减小/1 失败、转换增大、下一批 pending、按字节汇总；明确演示标记。
- 生产构建 4173 验证对比加载、前后同一几何坐标、100% 原尺寸 800×600、适应 552px、Esc 返回焦点恢复。浏览器对比明确使用同图模拟，不能作为原生编码质量验收。
- 预设草稿键盘调 level、切档放弃确认、另存并同步选中、只读 JSON 导出、设置 Tabs、未保存离开确认均走通。未开启真实监听。
- 最终生产预览 console 0 errors / 0 warnings，network 无失败。开发中一次金样 URL 请求超出 Vite allow list 被拒，未放宽文件访问；正式渲染使用 File 输入与 object URL。

### 原生投递与内存实测

构建产物直接启动，使用项目临时目录隔离 `YALI_HOME/settings.json`，clipboard/watch/tray/notify 均关闭。两次 `open -a Yali.app` 各投 5 张金样 JPEG 副本，**10/10 真实产物**；日志确认两次投递，不依赖浏览器模拟。

- 首轮：6.10s，44 次 RSS 采样，主进程峰值 **100.08MiB**，可观察子进程树 **162.75MiB**。
- 收口构建复核：6.17s，43 次采样，主进程 **101.92MiB**，可观察子进程树 **127.30MiB**。
- `ps` 不能完整归属 WebKit XPC，不作为完整应用峰值证据。以上资源与耗时仅作观察，不设 <300MB 硬性验收，也不将 Instruments 抽验列为交付阻塞。
- 测试应用结束后停止，仅临时副本有产物。按用户明确确认清理 `.playwright-mcp/`、`.pnpm-store/`；此处保留量化证据，正式 `design/` 原图不删除。

### 仍待人工/补充验收

- [ ] 真 Tauri 原生多选、取消选择、自定义/监视目录选择、窗口拖入到 pending、修改草稿后真实处理、Finder 定位、多目录菜单及原生重名冲突提示。
- [ ] default/jpg 更新影响提示、导入覆盖/失败保留、剪贴板复制失败回退的原生路径，用户实际使用确认。
- [ ] macOS WKWebView 的 100%/适应/分割滑杆、长图与尺寸变化并排、HEIC/JXL 不可直接显示时的预览、键盘与窗口缩放手感。
- [ ] 设置保存与后台应用实际生效；不自动开启监听验收。

无分支、commit、push；用户未亲验项不勾选。

### 用户补充调整与验证（2026-11-18）

- 耗时与系统资源改为观察项；同步设计、流程、状态和历史任务卡的现行验收口径。历史实测和旧脚本退出码保留，不重写基线。
- README 面向使用者，补齐当前能力、环境、运行与构建说明；AGENTS 只留执行约束、验证命令和按需阅读入口。项目 CodeGraph 规则要求先查具体代码，已返回源码不重复读取，缺失再定向查找。
- 新增 `scripts/dev.sh`（原生开发 / `--web` 演示）与 `scripts/build.sh`（release 应用 + bundle 校验），支持仓库外调用。保持现有 Tauri 前端钩子，避免重复构建；KISS：只封装现有命令，不安装依赖或添加发布流程。

验证结果：

```text
bash -n scripts/dev.sh scripts/build.sh benchkit/bench.sh：通过。
仓库外调用 dev.sh --web：Vite 启动，HTTP 页面入口有效；验证后停止。
dev.sh 原生调度：替身命令确认 cwd 与 CI 处理；本轮不以此替代原生人工验收。
两脚本：help、非法参数、缺依赖路径通过；构建失败返回原退出码，未校验旧包。
build.sh 相对 CARGO_TARGET_DIR：替身命令确认已转换为绝对路径。
仓库外调用 build.sh：真实 UI / release / .app 构建成功，14.75 MiB；10 类图片、10 个具体 UTI 校验通过。
金样判定 6 组受控输入：耗时翻倍 exit 0；体积 / SSIM 劣化、照片绝对质量门仍 exit 1。
benchkit/bench.sh --profile default：exit 0；体积 / SSIM 无劣化。
  3 次耗时中位 ms：photo 95 / ui 50 / palette 344 / GIF 159 / transparent 46 / HEIC 174。
  HEIC 耗时 +6.7% 只输出观察提示，不改变退出码；基线未改。
```

### 审查修复与复验

- 复现并修复：同路径已有 pending 时重试新 attempt 被 add 去重丢弃；改为 submit 携带 jobs 原子入队，原 pending 与旧结果保留。
- 复现并修复：TIFF 自动输出实际为 JPEG，原重名预检漏判；现按实际格式族检查，含 tif 别名。
- 两个回归先红（2 failed / 59 passed）后绿（61 passed）；安全与架构复核未发现修复引入的新具体问题。
- 隔离副本：`pnpm --dir ui test` 9 files / 61 passed；`pnpm --dir ui build` 成功；`cargo test --locked --offline -p yali-app` 20 passed。
- 浏览器实际操作：失败 → 同文件重新添加 pending → 重试旧失败项，新 attempt 执行至终态，原 pending 保留；console 0 errors / 0 warnings。此为演示路径，不代替原生人工验收。
- 原工作区 `bash scripts/build.sh` 再次成功，Yali.app 14.75 MiB，10 类图片 / 10 个具体 UTI 校验通过。
- 文档已按用途精简，纠正实施状态、设置默认值与规划能力混写；设计原图、ADR 理由、历史测量和人工待验项保留。
- `bash benchkit/bench.sh --profile default`：沙盒内 sips 写 HEIC 报 Error 13；沙盒外完整通过，6 张样图体积比 / SSIM 均在阈值内，基线未改。耗时只作观察。
