# T-023 · 自包含 App、内嵌 CLI 与右键操作管理

status: 已实施，待人工验收（2026-11-19）
goal: 最终 Yali.app 在无 Homebrew、无源码目录的环境中，GUI、CLI 和 Finder 快速操作均能真实处理图片
manual-check: true

## 已批准范围

- GUI / CLI 继续共用 yali-core，CLI、当前图片引擎及完整动态库依赖随 App 分发。
- 构建产物为运行时唯一来源，配置继续使用 `~/.yali/` / `YALI_HOME`；稳定 CLI 入口指向 App，不复制一套独立运行时。
- 设置页管理两条 Finder 快速操作，安装 / 修复 / 卸载仅处理本应用文件；公开 API 刷新 Services，系统禁用状态由用户启用，不改写 pbs 私有偏好。
- 个人学习版，使用本机 ad-hoc 签名，不要求 Apple Developer 证书、公证或商业发布。系统首次打开限制保留。
- 首批验证本机 Apple Silicon；不承诺未经验证的 Intel / 低版本系统支持。
- 用户已授权上述实现；不创建分支、commit 或 push，不改动与此任务无关的数据。

## 验证计划

- core / app / UI 测试，UI 构建、整包构建、bundle UTI 与全部动态依赖校验、ad-hoc 签名验证。
- 复制最终 App 到独立目录，以隔离数据根、最简 PATH 和禁止访问 Homebrew / 源码的沙盒运行 CLI 与真实 workflow，覆盖四个预设及图片格式。
- GUI 原生运行与右键菜单可见性另记实际验证边界；未经用户亲验继续标待人工验收。
- 压缩金样使用重建 CLI，质量门槛与基线保持不变。

## 实现

- `scripts/build.sh` 构建 release CLI，`package-runtime.py` 收集 Tauri sidecar 与递归 dylib 依赖、移除外部 RPATH、改包内相对引用，嵌入第三方许可证 / SBOM / Homebrew 配方，最后由内向外 ad-hoc 签名。
- 13 个工具（含 yali 与 ffmpeg）、97 个动态库 / 插件，总计 111 个 Mach-O；工具在 MacOS，库和 HEIF / JXL 插件在 Frameworks。Resources 的 vips ABI 路径通过包内相对链接定位，规避带句点代码目录被 codesign 误认为 bundle 的规则。
- core runtime 通过当前可执行文件和清单定位完整 App，符号链接启动也适用。包内模式禁止外部引擎与 Homebrew 回落；开发模式维持原查找策略。引擎环境只设置在子进程，不改全局 PATH。
- 两个 workflow 编译期嵌入 core；设置页与 `yali services` 共用状态 / 安装 / 卸载。串行文件锁、全部预检、暂存发布和失败恢复；同名非托管文件不覆盖。卸载保留数据与 CLI 链接，启动时仅修复已有链接。
- AppKit 在 CLI 退出时会输出 XPC 日志，实际污染 JSON；公开 Services 刷新在同一 CLI 的隐藏子命令中执行并捕获日志，安装 / 卸载继续返回纯 JSON。GUI 直接调用公开刷新 API。
- 隔离实测发现原先声明 BMP 但 sniff 未识别；补齐 BMP → 系统 ImageIO PNG → oxipng 无损 PNG，沿用共享管道，UI 输出冲突检查同步识别 BMP → PNG。无 Node/SVGO 依赖，不再在右键中声明 SVG。
- 对应架构与路径决策见 ADR-006。未创建平台集成 crate 或 Swift helper；未创建分支、提交或推送。

## 已完成验证

```text
cargo test -p yali-core -p yali-app：86 passed（6 suites）。
pnpm --dir ui test：9 files / 64 passed。
bash scripts/build.sh：CLI、真实 UI、release App、自包含运行时和 ad-hoc 签名成功。
bundle-check：10 类图片 / 10 个具体 UTI，111 个 Mach-O 的动态依赖均在 App / 系统目录，codesign --verify --deep --strict 通过。
python3 scripts/smoke-bundle.py --keep：通过；Homebrew / 源码 / 原账户禁读，四个预设与格式、实际 workflow、安装 / 重装 / 卸载、ffmpeg SSIM=1.000000 通过。
YALI_BIN=<App>/Contents/MacOS/yali bash benchkit/bench.sh --profile default：六张金样体积比 / SSIM 通过，未改基线。性能仅观察。
```

浏览器生产预览验证安装、取消卸载不调用卸载、确认卸载与状态更新；640px、390px / 200% 字号无横向溢出，操作按钮换行，底部保存可达；console 0 errors / 0 warnings。浏览器通过模拟 Tauri bridge 验证交互，不冒充系统安装。

原生隔离副本通过 `sandbox-exec` 启动，PID 44182；使用系统打开文件事件投递临时 JPEG，GUI 真实生成 49,178-byte JPEG（原图 531,019 bytes，产物 SHA-256 `1d8e313e88661114d782ccb9144cf42ab26e56cbf36a37d598c8a08a8d717796`），日志记录同一输入。通过原生文件选择器加入临时透明 PNG，再点击“压一下 · 1 张”，产物 5,276 bytes（原图 32,978 bytes）。原生设置 → 自动化显示真实安装状态，点击修复后状态仍为两条已安装。上述为自动化实测，不计用户人工验收。

原生设置取消卸载后，测试 workflow 实际保留；确认卸载后界面返回“尚未安装右键操作。”，随后点击安装返回“两条已安装”。安装、修复、取消、确认卸载和重装均通过真实 Tauri command 执行，而非浏览器模拟。

KISS / DRY：保留同一 `yali-core` 与两条模板，GUI / CLI 共用 Services 文件管理，不引入额外常驻服务、Node 运行时或独立 CLI 算法。单一职责：运行时定位、系统集成、构建期依赖收集各自收口；图像算法沿用既有管道，仅补齐已声明的 BMP 输入。

最终 App 位于 `target/release/bundle/macos/Yali.app`，文件总量约 108,925,241 bytes（约 104 MiB；文件系统占用约 105 MiB）。Tauri 自身日志的 22 MiB 是注入动态库之前的体积，不能作为最终包体积。

## 验收边界与待验

- 原账户没有切换成新 macOS 登录账户；依赖隔离使用复制的完整 App、隔离 HOME / YALI_HOME、最简 PATH 和 sandbox-exec 禁止读写 Homebrew / 原仓库 / 原账户，并先实测 cat 被拒绝。
- Automator.framework 明确拒绝沙盒进程运行 workflow。workflow 的 shell 节点在强隔离沙盒运行；完整 Automator workflow 必须在正常系统上下文执行，使用同一临时 App 内 CLI。两者的证据分别记录。
- 本机 Homebrew 依赖实际最低部署目标最高为 macOS 27.2，App 如实声明 27.2；当前只验证 Apple Silicon。没有声称支持旧系统、Intel 或 Gatekeeper / 公证通过。
- [ ] Finder 菜单可见性、系统设置启用与原生安装 / 卸载手感待用户亲验。
