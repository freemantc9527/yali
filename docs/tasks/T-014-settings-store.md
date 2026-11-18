# T-014 · 应用设置存储（settings.json）

goal: 给 M2 各入口一个统一的持久配置家（watch 目录 / 剪贴板开关 / 托盘开关 / 通知开关）
context: `docs/DESIGN.md` §4（剪贴板、watch、tray 都要开关与目录列表）；现状只有 `ProfileStore`（压图参数），入口配置无处可放
constraints:
  - **零新增依赖**（serde / serde_json / anyhow）
  - camelCase + `deny_unknown_fields`（与 Profile 一致：拼错字段必须报错，不能静默忽略）
  - 缺字段回落默认（老版本配置可前向兼容），默认值 = 剪贴板 / watch 默认关闭，托盘 / 通知默认开启（不偷偷监听剪贴板/目录）
  - 文件损坏不崩：备份为 `settings.json.bad-<时间戳>` 后回落默认值
  - 原子写（临时文件 + rename）；`YALI_HOME` 重定向与 ProfileStore 同源
acceptance:
  - `cargo test -p yali-core settings` 全绿：往返 / 缺字段默认 / 未知字段拒绝 / 损坏回落并备份 /
    原子写无残留 / `YALI_HOME` 生效
  - `yali settings show` 输出当前配置 JSON；`yali settings path` 打印绝对路径；
    `yali settings reset` 备份旧文件后写回默认
  - Tauri `settings_show` / `settings_save` 命令 + command 冒烟测试
  - 文档：`docs/DESIGN.md` 数据段注明配置文件位置与版本字段
estimated: 1h
manual-check: false

## 数据结构（v1）

```json
{
  "schemaVersion": 1,
  "clipboard": { "enabled": false, "minBytes": 65536 },
  "watch": { "enabled": false, "dirs": [], "settleMs": 800 },
  "tray": { "enabled": true },
  "notify": { "enabled": true }
}
```

- `minBytes`：剪贴板里的小图（表情、截图碎片）不值得压缩，默认 64KB 以下忽略
- `settleMs`：watch 的防抖窗口，文件写完再入队（FSEvents 会连续触发）
- 输出目录排除不落配置：由 Profile 的 `output.dir/dirPath` 推导，避免两份事实源

## 验收记录（2026-11-17）

- `cargo test -p yali-core settings`：11 passed（默认监听关闭 / 缺失文件 / 往返 / 缺字段默认 /
  拼错字段报错且不删文件 / 语法坏→`.bad-<ms>` 备份+默认 / 未来版本拒绝 / 相对与重复目录拒绝 /
  原子写无残留 / reset 备份 / YALI_HOME）
- CLI（YALI_HOME 指向临时目录）：
  - `yali settings path` → `<tmp>/settings.json`
  - `yali settings show` → 默认 JSON（监听关闭，托盘 / 通知开启，minBytes 65536，settleMs 800）
  - `yali settings reset` → 「已恢复默认配置」+ 默认 JSON，目录只剩 settings.json
- Tauri `settings_show` / `settings_save` 已接入，command 冒烟覆盖：默认→保存→读回→非法版本拒写
- 文档：DESIGN §2 记录位置与异常策略；§6 数据段补充 settings.json
