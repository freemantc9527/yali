# T-002 · Profile Schema 与 Store

goal: Profile v1 JSON Schema + fixtures + yali-core 加载/保存/校验/迁移
context: docs/DESIGN.md §2（字段权威定义）
constraints:
  - 纯 Rust crate（yali-core），零 UI 依赖
  - 不新增第三方依赖（serde/serde_json/anyhow 已在白名单）
  - camelCase 序列化 + `deny_unknown_fields`；数据目录 `~/Library/Application Support/app.yali/profiles/*.json`，测试用 `YALI_HOME` 重定向
  - schema/profile.schema.json 是**文档与外部校验器的事实源**；Rust 侧靠 serde 强类型校验，不引 jsonschema crate
acceptance:
  - `cargo test -p yali-core profile` 绿
  - 五档出厂 fixtures（default / web / archive / mail / screenshot）全部能反序列化，且「读入→再序列化→与原文件逐字段一致」的 roundtrip 测试通过
  - 非法 JSON 报错含行号；未知字段报错并指出字段名
  - `yali profiles list` 列出磁盘上所有档；`yali profiles show <name>` 从磁盘读取（不再内置硬编码）
  - CLI 输出与 fixtures/default.json 逐字段一致
estimated: 2h
manual-check: false

## 实现要点
- `crates/yali-core/src/profile.rs`：现有骨架已含 `Profile`（camelCase roundtrip 测试在），按 DESIGN §2 扩全六段（pipeline/resize/quality/format/metadata/output/ui）
- 新增 `store.rs`：`ProfileStore::dir()`（尊重 `YALI_HOME`）、`load(name)`、`save(&Profile)` 原子写（写临时文件 + rename）、`list()`
- fixtures 即出厂档初始值：直接把 DESIGN §2 的出厂档描述落成 JSON（default=level 4；web=webp q75/avif 智能；archive=JXL 无损包裹；mail=targetBytes 500KB；screenshot=无损 WebP/oxipng）
- quality.kind 三态：level(1-10) / targetBytes / lossless；`minQualityFloor` 只在 targetBytes 下生效
- 完成后：更新 STATE.md 勾选 T-002、建 `docs/tasks/` 索引
