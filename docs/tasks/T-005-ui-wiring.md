# T-005 · UI 接线（脚手架收尾）

goal: 参数控制台最小可用版——Profile 读取 + 拖入文件 + 调用内核 compress + 任务列表
context: docs/DESIGN.md §5（GUI 规格，本卡只实现其中的「读」与「跑」，不做完整视觉）；src-tauri/src/lib.rs（已注册 core_version）
constraints:
  - **禁止无卡样式迭代**：只按本卡组件清单做，不顺手美化
  - UI 不直接实现压缩逻辑，全部经 `invoke` 走 Rust command
  - Tailwind 是否引入：本卡不引（纯 CSS/内联即可），M1 视觉任务卡再评估
acceptance:
  - `pnpm --dir ui test` 绿（新增组件的纯逻辑单测）
  - `cargo tauri dev` 手动冒烟（**manual-check**）：
    - [ ] 显示内核版本（来自 invoke，非硬编码）
    - [ ] Profile 下拉列出磁盘五档，切换后表单字段跟随
    - [ ] 拖入 jpg → 调 compress → 列表行出现 `766KB → 334KB (-56.2%)` 样式的结果
    - [ ] 负优化件显示 ⚠ 状态
  - 验收输出贴进 commit message（截图可放 PR/issue，不入库）
estimated: 3h
manual-check: true

## 实现要点
- src-tauri 新增 commands：`profiles_list` / `profiles_show` / `compress(paths, profile_name)`（复用 T-002 Store 与 T-003 内核，壳层零业务逻辑）
- ui/src：`App.tsx` 拆出 `ProfileForm` / `TaskList` / `DropZone`；`bridge.ts` 保持「浏览器降级」模式（无 Tauri 时返回占位，便于 vitest）
- 拖入的文件路径由前端传字符串路径（Tauri file-drop 事件），内核负责真实 I/O
- 完成后：STATE.md 勾 T-005 并把「M0 完成」写入；`docs/STATE.md（当时的 M0 计划已收口删除）` 归档删除
