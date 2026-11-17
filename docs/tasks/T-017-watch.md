# T-017 · watch 文件夹（拖进目录即压）

goal: 监视指定目录，新图片落进去就自动压缩（输出同目录，`-{profile}` 后缀），不误伤自己的产物
context: `docs/DESIGN.md` §4「watch 文件夹 | notify crate（FSEvents），(path,mtime,size) 去重」；`settings.watch`（T-014）
constraints:
  - 新增依赖 `notify 8`（CC0-1.0，无 GPL 传染）；递归监视（FSEvents）
  - **防自激**：产物落在被监视目录里，必须进 ignored 集，否则 `-default-default-default` 无限套娃
  - **防抖**：一个文件写入触发多次事件 → `settleMs`（默认 800ms）窗口结束才处理
  - **去重**：(path, mtime, size) 相同只处理一次；文件被覆盖（mtime/size 变）才重压
  - 忽略隐藏文件、`.yali-undo.json`、管道中转名（`.yali-<pid>.tmp.`）
  - 目录/开关变更由 `settings_save` 就地生效，不重启（目录变了重建 watcher）
acceptance:
  - `cargo test -p yali-app watch`：候选过滤 / 抖动合并 / 同版本不重压 / 变更重压 / 配置映射
  - 端到端：被监视目录里放一张照片 → 出现 `*-default.*`；等 5s 无二次产物（自激已挡）
  - 同目录再放第二张 → 第二份产物；覆盖同名文件换格式 → 用新扩展名出新产物
estimated: 2h
manual-check: true（Finder 里拖图进目录是否 1–2s 内出产物；大文件写入过程中不产生半成品）

## 设计取舍

- 输出走 `default` 档的 sibling 策略：目录里的图压完还在原地（这是 watch 的自然语义）
- 压缩结果**不弹通知**：批量拖 30 张会炸通知中心；GUI 开着时行内进度足够，M3 再做聚合提示
- 引擎注册表用 `OnceCell` 缓存 + 首次预热：避免每张图都付 vips 冷启动

## 验收记录（2026-11-17）

- 依赖：`notify 8.2.0`（CC0-1.0）+ `notify-types 2.0.0`（MIT/Apache-2.0）
- `cargo test -p yali-app`：18 passed（watch 4 + clipboard 8 + tray 3 + open-file 2 + 冒烟 1），零警告
- 端到端（`YALI_HOME` 临时目录，watch 指向 `/tmp/yali-watch-test`，settle 800ms）：
  - `inbox-1.jpg`（531019 B）→ `inbox-1-default.jpg`（49178 B）
  - 等 5 秒：目录里没有 `*-default-default*` ⇒ 自激被挡
  - `inbox-2.png`（34757 B）→ `inbox-2-default.png`（4696 B）
  - 用另一张图覆盖 `inbox-1.jpg`（内容与扩展名都变）→ 新增 `inbox-1-default.png`（5276 B）
- 未做：托盘菜单里的 watch 开关（托盘菜单目前只有「打开/退出」，开关留给 T-018 之后统一做）
