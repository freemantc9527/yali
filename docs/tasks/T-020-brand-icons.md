# T-020 · 品牌图标与菜单栏模板图

goal: 用「抱梨的鸭」替换占位字标，保留可复用主体，补齐应用图标集与单色菜单栏图标
context: `docs/NAMING.md` §1 / §4；T-013 应用打包；T-015 托盘
constraints:
  - `mascot.png` 保留 1024×1024 透明主体，供后续复用
  - 应用图标使用苔绿色圆角背景，外侧透明，主体保留安全边距
  - 菜单栏图标从 mascot 位图经 ImageMagick 提取，不重新绘制；单色、透明、无边框
  - 只保留当前 bundle 配置所需资源；移动端 / Appx 派生资源不入库
acceptance:
  - `cargo test --locked -p yali-app`：18 passed
  - `env -u CI cargo tauri build --bundles app` 产出 Yali.app
  - `scripts/bundle-check.sh`：10 类图片、10 个具体 UTI 校验通过
  - 包内 icon.icns 与源文件 SHA-256 一致
manual-check: true（应用与菜单栏图标的显示、尺寸和辨识度）

## 资源与接线

- `src-tauri/icons/mascot.png`：1024×1024 透明吉祥物。
- `src-tauri/icons/icon.png`：1024×1024 应用母版，ImageMagick 合成苔绿色圆角背景。
- `32x32.png / 128x128.png / 128x128@2x.png / icon.icns / icon.ico`：由母版经
  `cargo tauri icon` 在临时目录派生，只将配置引用的五项复制回项目，母版不被生成器覆盖。
- `yaliTemplate.png`：36×36 黑色与透明度模板，图案有效范围 34×33；由原图亮暗分区提取透明间隙，
  保留鸭与梨之间的分隔。托盘使用 `Image::from_bytes` 解码，`icon_as_template(true)` 随系统外观着色。
- Tauri 开启 `image-png`，锁文件同步其 PNG 解码依赖；取代 T-015 初版复用应用彩色图标的方案。

## 验收记录（2026-11-18）

- `cargo test --locked -p yali-app`：18 passed，零失败。
- `env -u CI cargo tauri build --bundles app`：成功，Yali.app 12.56 MiB。
- `scripts/bundle-check.sh`：10 类图片、10 个具体 UTI，无抽象类型。
- 已启动构建产物；用户确认放大后的菜单栏图标及本次图标人工核验通过。
- 设计比较稿、抠图中间件、截图与临时派生资源清理，不入库。
