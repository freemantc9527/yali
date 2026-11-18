# T-015 · 菜单栏托盘 + 关窗常驻

goal: 让鸭梨成为一个常驻 app：菜单栏有图标，关窗只是收起来，为剪贴板/watch 入口提供挂载点
context: `docs/DESIGN.md` §4 入口矩阵；`docs/STATE.md` §4 第 2 条；`settings.tray.enabled`（T-014 已定义，默认 true）
constraints:
  - 只开 `tauri` 的 `tray-icon` feature，不引新 crate
  - 关窗行为必须跟随 `settings.tray.enabled`：托盘关着就按普通 app 退出，不许出现「窗口没了进程还在但无处唤回」
  - 初版图标复用 `default_window_icon()`；T-020 已改为独立单色模板图，随系统明暗着色
  - 菜单只放「打开鸭梨 / 退出鸭梨」，剪贴板/watch 开关等 T-016/T-017 有了再加
acceptance:
  - `cargo test -p yali-app` 覆盖 `close_action(tray)` 三态与菜单 id 前缀
  - 启动后进程存活，System Events 能读到 `menu bar 2` 的 status menu（托盘存在）
  - 点窗口关闭按钮 → 窗口数 0 且**进程仍存活**（自动验证）
  - `settings.tray.enabled=false` 时关窗必须退出进程（同一套 `close_action` 决定）
estimated: 1h
manual-check: true（菜单栏图标是否可见、点图标是否弹菜单、菜单项是否把窗口唤回）

## 实现要点

- `src-tauri/src/tray.rs` 独立模块：`apply(app, enabled)` 幂等（已建则只切可见性）
- 运行期设置放 `RuntimeSettings(RwLock<Settings>)`：`settings_save` 落盘后同步内存并即时切托盘，
  不重启也生效
- 关窗拦截用 `WindowEvent::CloseRequested` + `api.prevent_close()` + `window.hide()`；
  `try_state` 兜底（窗口早于 setup 创建时按无托盘处理）
- 托盘菜单 id 统一 `yali.tray.*`，与 webview 事件名（`yali://…`）分区

## 验收记录（2026-11-17）

- `cargo test -p yali-app`：6 passed（tray 3 + open-file 2 + command 冒烟 1），零警告
- 整包：`env -u CI cargo tauri build --bundles app` → Yali.app 11.19 MiB
- 启动后 `System Events` 读到 `menu bar 2` = `status menu`（托盘状态项存在）
- AX 点击窗口关闭按钮 → `count windows` = 0，`pgrep` 仍见 `Yali.app/Contents/MacOS/yali-app`
  ⇒ 关窗收进托盘、进程常驻，自动验证通过
- 菜单展开与「打开鸭梨」唤回窗口需真实鼠标点击，AX 点 status item 拿不到菜单层级 → 留给人工
