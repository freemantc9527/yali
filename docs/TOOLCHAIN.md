# TOOLCHAIN · CLT-only 构建边界

本项目仅依赖 macOS Command Line Tools，不依赖完整 Xcode。[ADR-002](decisions/ADR-002-tauri-clt-only.md) 记录 Tauri 选型，[ADR-003](decisions/ADR-003-contingency-appkit-shell.md) 是未触发的兜底方案。2026-11-17 已实证 `cargo build -p yali-app` 与 `cargo test` 可用，不重复探测。

## 1. 环境与命令

Rust + Tauri 2 + React / Vite；前端使用 pnpm。环境安装与脚本见 [README](../README.md)，验证命令见 [AGENTS](../AGENTS.md)。Tauri CLI 通过 `cargo install tauri-cli --version '^2' --locked` 安装，本机实测版本 2.12.5。

| 能力 | CLT-only 边界 |
|---|---|
| swiftc / swift build / swift run | 已实证可用；仅用于任务批准的独立 helpers，当前目录尚未创建 |
| swift test | 缺 XCTest / swift-testing，不可用，禁止作为验收命令 |
| SwiftUI 宏 | 缺 SwiftUIMacros，不能用 @State / @Observable 等宏作为主栈 |
| AppKit / Vision / ImageIO / UTType | 小工具编译与运行已实证；AppIntents 仅 typecheck |
| ad-hoc codesign | 可用，开发与 workflow 不需证书 |
| Developer ID / notarytool | CLT 有工具；证书延后，未交付签名分发 |
| workflow | 文本模板安装，免 Xcode / 签名 |
| xcodebuild / actool / Mac App Store | 不在项目工具链与路线内 |

## 2. 构建与 Tauri 易错点

- `before*Command` 的 cwd 是 `src-tauri`，钩子用 `pnpm --dir ../ui …`。
- `CI=1` 会使 cargo-tauri 的布尔参数解析失败；dev / build 脚本统一 `env -u CI`。
- `generate_context!` 编译期需要 `src-tauri/icons/icon.png`；图标必须 RGBA8，gray+alpha 会启动 panic。
- 非 dev 构建硬校验 `frontendDist=../ui/dist`；build.rs 只提供编译占位，整包验收必须有真实 UI。debug 成功不能证明 release 可复现。
- `capabilities/default.json` 须保留 `core:default`，否则 listen 被 ACL 拒绝，进度 / open-files 事件会静默失效。
- `bundle.macOS.infoPlist` 路径相对 src-tauri，顶层键整键覆盖。用精确 CFBundleDocumentTypes 覆盖 bundler 对 AVIF / JXL 的 public.image 兜底，`bundle-check.sh` 防漂移。
- 应用包固定 ASCII `Yali.app`；中文显示通过 CFBundleDisplayName 本地化。
- 迁移仓库路径后若指纹仍含旧绝对路径，需在授权范围内清理对应编译缓存再重建，不能拿旧产物验收。
- 16GB 机器可限制 Rust 并发 `-j4`；资源与耗时仅观察。CI 使用相同 CLT-only 命令，不依赖 runner 的完整 Xcode。

## 3. 引擎与系统约束

ADR-006：分享包必须用 `bash scripts/build.sh` 生成。脚本先构建 release CLI，递归收集引擎 / dylib 与 libvips HEIF、JXL 插件，重定位到包内 `@loader_path`，使用 Tauri `externalBin` 携带工具，再从内向外 ad-hoc 签名。`scripts/package-runtime.py check` 校验所有动态依赖不指向 Homebrew / 源码目录。裸 `cargo tauri build` 不再作为自包含分享包验收。

构建输入位于 `${CARGO_TARGET_DIR:-target}/yali-bundle-input/`，不进版本库；运行时只依赖最终 App。`Resources/yali-runtime.json` 记录架构与所有依赖的最高部署目标，Info.plist 的 LSMinimumSystemVersion 跟随该实际要求。本机依赖最高为 macOS 27.2，当前仅验证 Apple Silicon；需要兼容 macOS 14 时须针对该部署目标重新构建引擎，而非降低 plist 声明。

当前 App 只携带图片处理所需工具，不再打包 FFmpeg 及其独占动态依赖。图片引擎在包内缺件时仍禁止回落开发机；可选外部 FFmpeg 是明确例外，从 PATH 或 macOS 标准 Homebrew 路径查找，用于 SSIM 和额外解码校验。设置页可重新检查路径与滤镜，结果不写入配置。benchkit 仍需开发机安装 FFmpeg 进行质量度量。

Windows 已通过首轮云端 App / CLI 编译、UI 测试和无引擎 ZIP 版本检查；首轮 Rust 测试尚未执行，图片运行时与真实 Windows GUI 未验证。新增 windows-runtime.py 的 prepare/package/check、build-windows.ps1 和 smoke-windows.py 已完成本地实现及脚本测试，仍待本轮 Windows runner 构建与运行验证。直接依赖 dirs 7.0.0 与 sha2 0.10.9 复用既有依赖树，分别用于 Windows Known Folder 和包内摘要检查。本机无 Windows 工具链和运行环境，不能用 macOS 构建通过推定 Windows 可用，见 [CROSSPLATFORM](CROSSPLATFORM.md)。

个人学习版不要求 Developer ID / 公证。首次打开优先使用 Finder 的“打开”或“系统设置 → 隐私与安全性 → 仍要打开”；如果下载包仍被隔离属性拦截，可以只对目标 App 执行 `xattr -dr com.apple.quarantine "/Applications/Yali.app"`，然后用 `open "/Applications/Yali.app"` 启动。`codesign --verify --deep --strict` 只能验证本机 ad-hoc 完整性，不能据此声称通过 Gatekeeper 或公证。

引擎清单以 [Brewfile](../Brewfile) 为准，具体安装版本以 doctor 为准。mozjpeg 是 keg-only，PATH 中的 cjpeg 可能来自 libjpeg-turbo；运行时探测 `/opt/homebrew/opt/<pkg>` 与 `/usr/local/opt/<pkg>`，不调用可能联网挂起的 brew --prefix。图像处理与度量坑见 [benchkit](../benchkit/README.md)。

NSPasteboard 只在主线程访问，setup 不能 block_on 等主线程回调。剪贴板按 changeCount 和内容摘要去重；watch 忽略自身产物并按 settleMs + (mtime, size) 去重。Finder Services 安装与 pbs 类型约束见 [workflows](../workflows/README.md)。
