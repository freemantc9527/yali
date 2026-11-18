# T-016 · 剪贴板监听（复制即压）

goal: 复制一张图 → 自动压缩 → 存到「下载」并通知；菜单栏 app 常驻时也工作
context: `docs/DESIGN.md` §4 入口矩阵「剪贴板监听」；`settings.clipboard`（T-014）
decision:
  - **不用 arboard**：macOS 只给原始 RGBA，还得自带 PNG 编码器；NSPasteboard 直接给 `public.png`
    （编码后的字节，`minBytes` 体积语义才对得上）。objc2 / objc2-app-kit 本就是 Tauri 的传递依赖
  - **产物落「下载」**（`DirKind::Downloads`）、不带后缀、不写撤销清单：剪贴板源图是临时文件，撤销无意义
  - 只监听图片；小于 `minBytes`（默认 64KB）的图跳过（表情、图标不值得压）
constraints:
  - NSPasteboard **只能在主线程**访问 → 轮询经 `run_on_main_thread`，压缩在后台任务
  - **启动时先 `attach` 对齐 changeCount**：不把启动前就在剪贴板里的图当新图
  - changeCount 变了但内容相同（同一张图复制两次）不重复压：FNV-1a 摘要去重
  - 有些 app 只放 TIFF → `sips -s format png` 转一次再压，不丢图
  - 临时 PNG 放 `~/Library/Caches/app.yali/clipboard/`（`YALI_HOME` 可重定向），处理完即删
acceptance:
  - `cargo test -p yali-app`：watcher 状态机 6 测（attach/同 count/小图/重复内容/空剪贴板/摘要）+ TIFF 转换 + 输出档
  - 端到端：设置 `clipboard.enabled=true` → 复制 1000×1000 PNG → `~/Downloads/clip-*.png` 出现且尺寸不变
  - 重复复制同一张图 → 不产生第二个文件
estimated: 2h
manual-check: true（通知文案是否出现、macOS 是否弹剪贴板隐私提示）

## 验收记录（2026-11-17）

- 依赖：`objc2 0.6` + `objc2-app-kit 0.3`（feature `NSPasteboard`）+ `objc2-foundation 0.3`（`NSData`），
  均为 Tauri 既有传递依赖，只是提升为直接依赖
- `cargo test -p yali-app`：14 passed（clipboard 8 + tray 3 + open-file 2 + 冒烟 1），零警告
- 端到端（`YALI_HOME` 指向临时目录，`clipboard.enabled=true`、`minBytes=0`）：
  - 复制 `benchkit/samples/transparent.png`（32978 B, 1000×1000，带透明）
  - `~/Downloads/clip-1794904780652998000.png` 出现：5276 B，1000×1000 ⇒ **-84%，尺寸未变**
  - 再复制同一张图：文件数 1 → 1（摘要去重生效）
- macOS 未弹剪贴板读取授权框（本机实测）；若将来系统弹框，属系统策略不是 bug
- 已知取舍：压缩结果不回写剪贴板（避免偷偷改用户粘贴内容），需要回写再开卡
