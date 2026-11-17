# T-013 · 真 .app 打包 + CFBundleDocumentTypes + open-file 入队

goal: 产出可被 LaunchServices 认领的 `Yali.app`，把「打开方式 / Dock 拖入 / `open -a`」接进压缩队列
context: `docs/DESIGN.md` §4 入口矩阵（open-file 原挂 M1，实际未落地）；`docs/STATE.md` §4；`src-tauri/tauri.conf.json` 仍 `bundle.active=false`
constraints:
  - 无 Xcode、无 Developer ID：本机工具定位，ad-hoc 签名即可（证书仍延后）
  - UTI 必须与 T-012 同源的**具体类型**，禁用抽象 `public.image`（坑账本：抽象 UTI 在正常目录不匹配）
  - open-file 事件可能早于前端就绪 → 必须有 pending 缓冲，不能只在窗口里监听事件
  - 不改 UI 布局与视觉，只加事件接线（WORKFLOW §4）
acceptance:
  - `cargo tauri build` 产出 `target/release/bundle/macos/Yali.app`
  - `plutil -p Yali.app/Contents/Info.plist` 含 `CFBundleDocumentTypes`，`LSItemContentTypes` 覆盖
    `public.png / public.jpeg / public.heic / public.heif / com.compuserve.gif /
    org.webmproject.webp / public.avif / public.jpeg-xl / public.tiff / com.microsoft.bmp`
  - `open -a <abs>/Yali.app <sample>` 启动后待处理队列里有该文件（冷启动路径）
  - 单测：pending 缓冲 push/take、非图片被忽略、`file://` URL 解码（中文/空格路径）、重复投递去重
  - `cargo test` / `pnpm --dir ui test` 全绿零警告（数量记进 commit）
estimated: 2h
manual-check: true（Finder「打开方式」里出现 Yali；Dock 拖入图标入队）

## 实现要点

- `bundle.active=true` + `targets=["app"]`；`bundle.icon` 用 `cargo tauri icon` 生成的完整图标集
  （仓库现在只有 `icons/icon.png`，缺 `.icns` 会让 bundler 失败）
- `bundle.fileAssociations` 按扩展名声明，role=Viewer；生成后**必须回读 plist** 确认 Tauri 是否
  真的写了 `LSItemContentTypes`（只写 `CFBundleTypeExtensions` 时 Finder「打开方式」会漏项）
- `RunEvent::Opened { urls }` → 过滤存在且扩展名在白名单内的本地文件 → 存入 `PendingFiles`
  + `emit("yali://open-files")`；前端挂载时先 `take_pending_files()` 再订阅（冷启动不丢件）

## 验收记录（2026-11-17）

- 工具链：`cargo install tauri-cli --version '^2' --locked` → v2.12.5（CLT-only 编译 4m34s 通过）
- 图标：`cargo tauri icon src-tauri/icons/icon.png` → 32/128/128@2x + icon.icns + icon.ico
  （移动端/Windows 产物已删，macOS 先行）
- 整包：`env -u CI cargo tauri build --bundles app` →
  `target/release/bundle/macos/Yali.app`（10.80 MiB）
- `scripts/bundle-check.sh`：
  `✓ bundle 校验通过：10 类图片，10 个具体 UTI，无 abstract 类型`
  （com.compuserve.gif, com.microsoft.bmp, org.webmproject.webp, public.avif, public.heic,
  public.heif, public.jpeg, public.jpeg-xl, public.png, public.tiff）
- 真投递（端到端，压出产物才算过）：
  - 冷启动单文件：`open -a …/Yali.app photo.jpg` → `photo-default.jpg`，531019 → 49178 B
  - 运行中投递：`photo4.jpg` → `photo4-default.jpg`
  - 冷启动多文件：`multi-a.png` + `multi-b.gif` → 两个 `*-default` 产物
- 单测：`cargo test` 61 passed（core 53 + cli 3 + app 3 + fixtures 2）；`pnpm --dir ui test` 49 passed；零警告
- 顺带修掉三个必踩的坑：钩子 cwd 是 src-tauri（`--dir ../ui`）、`CI=1` 会打挂 cargo-tauri、
  **缺 capabilities 导致前端 listen 被 ACL 拒绝**（`yali://progress` 在真机上此前一直是哑的）

manual-check 未过：Finder「打开方式」是否出现 Yali、Dock 拖图标是否入队（需人眼）。
