# T-009 · 任务队列与进度事件

goal: 批处理并发（min(cores-1,4)）+ 逐文件进度事件，UI 不等整批返回才更新
context: docs/DESIGN.md §6 任务模型；docs/plans/PLAN-M1.md；T-003 并发上限
constraints:
  - 并发上限与 CLI 一致（min(cores-1, 4)）；不引新依赖（tokio 已有）
  - 事件只承载「进度」，结果权威仍是 command 返回值（事件丢失不影响正确性）
  - UI 用纯 reducer 处理事件（可单测，不依赖 Tauri）
acceptance:
  - Tauri compress 逐文件发 `yali://progress`（started/done/failed，含 index/total）
  - UI reducer 单测：started→running、done→report、failed→error、乱序事件不崩
  - 10 图批处理 UI 逐行更新（浏览器 fallback 模拟事件）
  - cargo/ui 测试全绿
estimated: 3h
manual-check: false
