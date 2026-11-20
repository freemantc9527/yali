# T-029 · 开箱即用的推荐压缩

status: 已实施并本地验证，待人工验收（2026-11-20）
goal: 固定保守参数配合无损优化，让普通用户无需调参即可压缩图片
manual-check: true

## 已批准范围

- 用户确认推荐方案并要求最小改动：JPEG q75 与无损候选择小；PNG quality 94–98 / speed 1 后接 oxipng -o 2；量化质量不满足时走无损。
- 使用现有编码器与共用内核，不新增依赖、分类器或运行时 SSIM 搜索。
- 新增 recommended 模式；只升级完全匹配旧出厂配置的 default / jpg / thumbnail，保留用户自定义与旧 level 语义。
- 质量设置收入现有高级折叠；四个用途、尺寸、透明、动画及原图保护不变。
- 本卡包含必要的 Schema、内核、预设、UI、测试及文档，超过 8 个文件；不提交、建分支、安装 Services 或发布。

## 验收

- 内核、壳层、UI 测试与前端构建；真实整包及 bundle 校验。
- 重建 CLI 后跑 default / jpg 六张金样，明确记录保真策略引起的体积变化后更新对应基线，再复跑。
- 用户三张本地素材仅在忽略目录 / 临时目录处理；验证尺寸、SSIM、原图哈希，个人产物不入库。
- 覆盖预设兼容、量化退出 99、后处理失败、无效候选、输出变大与缩放后的 JPEG 候选来源。
- 界面与肉眼画质待用户亲验，不以自动化代替人工验收。

## 首轮实测与基线调整依据

- UI 73 项测试与前端构建通过；core 73 项通过（新增质量拒绝回退、可选优化失败、无效候选、缩放与旧预设兼容）；app + CLI 26 项通过。壳层与金样首次受沙箱数据根 / sips 限制，隔离临时数据根并获准沙箱外复验通过。
- 三张用户素材：PNG 751788 bytes / SSIM 0.990739；JPEG 2 225781 / 1.0（jpegtran 保留标记）；JPEG 3 88741 / 0.993532。原图哈希未变，尺寸分别为 1024×1536、1706×1279、960×1440；产物仅在忽略的 target/t029-comparison。
- 旧 default 基线对照：photo 49178 → 79768 bytes、SSIM 0.964656 → 0.970086；transparent 5276 → 5473、0.932026 → 0.932151。UI PNG 4696 → 4489、0.999801 → 0.999881；palette 800966 → 802076、SSIM 仍 1；GIF / HEIC 体积与质量不变。
- 旧 jpg 基线对照：photo 同上；ui 13710 → 15530、SSIM 0.994731 → 0.996273；palette 49677 → 90626、0.895077 → 0.905197；transparent 11724 → 13969、0.913298 → 0.914403；HEIC 47549 → 77506、0.922609 → 0.934891；GIF 不变。
- 首轮完整基准明确因以上体积增长未通过旧基线，不能声称原基线通过。本卡有意提高默认保真程度，按已批准方案更新 default / jpg 对应基线，再复跑守护；不调整未涉及格式或手动参数映射。
- 首轮 default 推荐处理单图中位约 245–733ms（照片 346ms）；旧默认约 50–345ms。包含格式 / 尺寸探测、解码验证及候选优化，且两档并行执行、有其他测试负载；仅作观察，不据此做精确性能排名。

## 最终验证

- `cargo test --locked --offline -p yali-core -- --test-threads=1`：73 passed；`env YALI_HOME=<隔离临时根> cargo test --locked --offline -p yali-app -p yali-cli -- --test-threads=1`：26 passed（沙箱外复验）。
- `pnpm --dir ui test`：73 passed / 11 files；`pnpm --dir ui build` 与最终 `bash scripts/build.sh` 均成功，111 个 Mach-O 的依赖与 ad-hoc 签名、10 类图片 / UTI 通过。
- 重建 CLI 后，`bash benchkit/bench.sh --profile default --update-baseline` 与 `--profile jpg --update-baseline` 成功；随后分别不加更新参数复跑，六张体积比 / SSIM 均在新基线 3% 范围内，推荐档单代照片 SSIM≥0.95。
- 最终 App 临时副本在禁读 Homebrew、源码、原账户的 sandbox-exec 环境中真实处理六张金样与四预设；缩小尺寸 1024×683。只验证 CLI 与现有四用途，未安装 Services、未替换用户已安装 App；记录在忽略的 `target/t029-comparison/bundle-smoke.json`。
- 三张用户图显式切回旧 level 4 后，产物 SHA-256 与优化前结果一致；合成 APNG 从 1693650 → 868306 bytes，前后均 12 帧。数据在 `target/t029-comparison/legacy-check.json` / `animation-check.json`，个人产物不入库。
- 浏览器 DOM 验证推荐摘要、高级区默认 closed、质量控件默认隐藏，展开后可读取推荐模式，无水平溢出；前端模拟不算原生人工通过。
- `/usr/bin/time -l <最终 App 内 CLI> compress <三张用户图>`（沙箱外、M1 Pro / macOS 27.2.1 / arm64、3 图并发、一次热运行）：0.98s real / 1.65s user / 0.22s sys，maximum resident set size 48939008 bytes（约 46.7 MiB）。这是 time 的报告口径，不是对整个进程树同一时刻总内存的采样，也不是统计基准；三图 CLI 内报告 773 / 345 / 214ms。沙箱内资源读取曾因 sysctl kern.clockrate 受限退出，产物处理本身成功，以沙箱外完整观察为准。
- 原图未变；未新增依赖、提交、分支或远端操作。完整元数据策略仍沿用项目已记录的未验证边界，不宣称本轮完成颜色管理或隐私元数据专项。

## 待人工验收

- [ ] 打开本轮构建 App，检查三张图正常观看时的画质与颜色、透明图边缘，以及高级设置展开和模式切换。
- [ ] 微信 / Preview / QuickLook 查看产物的兼容性。只有用户亲验后才能标通过。

## 版本同步与提交

- 用户后续明确授权更新版本（含 App 页面）并提交。按 VERSIONING 将首次预览版定为 `0.1.0-alpha.1`，同步 workspace、Tauri、UI package 与 Cargo.lock 三个本地 package；外部依赖版本未变。
- 原生页面继续调用 core_version；浏览器预览读取 ui/package.json 的 version，避免另写展示版本。相应现有 UI 测试更新。
- 本次重新运行 Rust 三个 package 测试：99 passed / 9 suites；UI：73 passed / 11 files；发布脚本：14 passed；`python3 scripts/release.py check` 输出 version=0.1.0-alpha.1、prerelease=true。
- `bash scripts/build.sh` 成功；真实前端、111 个 Mach-O 依赖及签名、10 类 UTI 通过。包内 CLI 输出 `yali 0.1.0-alpha.1`；本地 Info.plist 的 CFBundleShortVersionString / CFBundleVersion 均由 Tauri 生成为 `0.1.0-alpha.1`，未手工修改签名包，不据此宣称 App Store 分发格式或正式发布门槛已通过。
- 包含本卡功能与必要版本同步，提交当前 main；不推送、不创建标签或 Release，不改变人工待验状态。
