# T-007 · Profile 编辑与保存

goal: GUI 表单可写：编辑六段字段 → 校验 → 原子落盘 → 回到只读视图；CLI 侧提供 headless 验收口
context: docs/DESIGN.md §2/§5；T-002 Store（原子写）
constraints:
  - 校验规则只有一份：UI 只做「提前提示」，落盘前以内核 `Profile::validate()` 为准（不引第二套规则）
  - 禁止新增前端依赖；表单不做视觉迭代（M1 视觉卡再统一）
  - 保存走既有原子写（Store::save），不新建文件格式
acceptance:
  - `echo '<json>' | yali profiles save <name>` 落盘，`yali profiles show <name>` 原样读回
  - 非法值（level=0 / targetBytes 缺失）在 CLI 报错且不落盘
  - Tauri command `profiles_save` 冒烟测试（临时 YALI_HOME）
  - `pnpm --dir ui test` 覆盖：字段更新 helpers + 表单错误提示（不依赖 Tauri）
estimated: 3h
manual-check: false

## 实现要点
- CLI：`profiles save <name>`（stdin JSON；`--file <path>` 也可）→ validate → store.save
- Tauri：`profiles_save(profile: Profile)` 复用 Store
- UI：`ProfileForm` 切到可编辑（受控输入）；`lib/profile-edit.ts` 纯函数（update 路径 + 局部校验 <20 行，最终以内核报错为准）
- 保存按钮状态机：idle → saving → saved/error（错误展示内核返回文案，不吞）
