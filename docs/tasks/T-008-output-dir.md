# T-008 · 输出目录策略（schema v1.1）

goal: output.dir 四策略真正生效（sibling / downloads / custom / override）+ `{size}` 占位渲染
context: docs/DESIGN.md §2 output；docs/plans/PLAN-M1.md
constraints:
  - schema v1.1：新增 `output.dirPath`（可空字符串），五档 fixtures 同步；`deny_unknown_fields` 保持
  - `dir=custom` 必填 dirPath，且必须是绝对路径或 `~` 开头（validate 拦）
  - `dir=override` 读环境变量 `YALI_OUTPUT_DIR`，未设置回落 sibling（workflow/CLI 单次覆盖语义）
  - `{size}` 在编码完成后渲染（编码前只用于临时命名，占位保留）
  - 不新增依赖；目录不存在时创建
acceptance:
  - cargo test -p yali-core：三策略 + `~` 展开 + 环境变量回落 + `{size}` 渲染 单测通过
  - CLI 实测：custom → `/tmp/out/photo-x-48K.jpg`；downloads（HOME 重定向）→ `$HOME/Downloads`；override（YALI_OUTPUT_DIR）→ 指定目录
  - undo 在 custom 目录下按清单还原
  - `pnpm --dir ui test` 覆盖 custom 必填路径
estimated: 2h
manual-check: false

## 实现要点
- `pipeline::output_dir()` + `final_path(..., size)` 两段式命名：编码前无 size（临时名），编码后按实际体积 rename
- **不要用 `brew --prefix` 兜底**：HOME 陌生时 brew 会拉 API 数据可挂死 → 直接探 `/opt/homebrew/opt/<pkg>`（坑账本）
- fixtures/schema/UI 类型/降级数据四处同步（`dirPath`）
