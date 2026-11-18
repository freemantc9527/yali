# T-011 · Profile 管理器（另存 / 导入 / 导出）

goal: GUI 里完成档位生命周期：基于当前档另存新档、粘贴 JSON 导入、导出当前档 JSON
context: 当前预设管理规格见 [UI-REDESIGN](../UI-REDESIGN.md)；T-002 Store；T-007 编辑
constraints:
  - 不引对话框插件（M1 不新增依赖）：导出 = 复制到剪贴板 + 展示文本；导入 = 粘贴文本
  - 档名规则：非空、不含 `/` `\`、不以 `.` 开头（与内核 path_for 一致）
  - 导入后必须过内核 validate（UI 侧只做 JSON.parse 与档名提示）
acceptance:
  - 纯函数单测：renameProfile / parseProfileJson（非法 JSON 带行列、非法档名拒绝）
  - 浏览器实测：另存为 → 列表出现新档；导入非法 JSON → 错误提示
  - command 层复用 profiles_save（已有），无新增 Rust 代码
estimated: 1.5h
manual-check: false
