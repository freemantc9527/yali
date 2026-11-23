# Yali 开发规则

本文件面向参与仓库工作的 Agent 和开发者。项目文档、代码和发布流程都以当前仓库内容为准，发现规则与实际实现不一致时，先修正文档再继续工作。

## 项目定位

Yali 是 macOS 优先的本地图片压缩与转换工具，提供 Finder 右键操作、桌面工作台和 `yali` CLI。Rust 内核、Tauri 壳层和 React UI 共用 Profile、数据目录与处理结果规则。

- 产品名：鸭梨（Yali）
- App bundle：`Yali.app`
- bundle identifier：`app.yali`
- CLI：`yali`
- 环境变量前缀：`YALI_`
- 数据根：默认 `~/.yali/`，可用 `YALI_HOME` 隔离
- 许可证：Apache-2.0

## 开始工作

1. 运行 `git status --short --branch`，确认当前分支和已有修改。
2. 阅读 `docs/STATE.md`、相关 `docs/` 设计文档和当前 `.tasks/` 任务卡。
3. 仓库存在 `.codegraph/` 时，理解或定位代码前先运行 `codegraph explore "具体问题或符号"`；查询结果已给出逐字源码的部分不重复读取。
4. 只做当前任务需要的改动。未经用户明确要求，不创建分支，不自动 pull，不执行 commit、push、合入或发布。
5. 保留用户已有的未提交修改，不使用 stash、reset、清理或覆盖来掩盖冲突。

## 分支与历史

- `main` 只保存可发布基线，禁止在其工作区直接开发或提交。
- 日常开发使用 `dev` 或按任务创建的短期分支。任务完成后先在开发分支验证，再按 [版本与发布规范](docs/VERSIONING.md) 合入。
- 新提交只解决一个可描述的问题，提交格式为 `type(scope): 中文结果说明`。提交和推送必须得到用户明确授权。
- 改写历史、强制推送、删除远端分支或标签属于高风险操作。执行前必须说明影响范围、回退办法和目标 refs，并取得明确的“是 / 确认 / 继续”。

## 任务与文档

开发任务卡只存在于被 Git 忽略的 `.tasks/`，不在 `docs/` 中长期保存。建议命名为 `.tasks/NNN-short-name.md`，至少包含目标、范围、验收条件、验证命令和风险。

任务完成时：

1. 在任务卡中记录实际命令、结果、未完成的人工验收和回退方式。
2. 把用户可见变化整理到 `docs/CHANGELOG.md` 的 `未发布` 或对应版本下。
3. 把当前事实更新到 `docs/STATE.md`，不要把整段过程复制进去。
4. 只有稳定、可复用的规则才进入 `docs/`；临时讨论、草稿和实验记录留在 `.tasks/`。

## 实施边界

- 配置、预设、撤销记录、缓存和日志统一通过 `yali_core::data` 使用数据根解析，迁移必须可重试并以原子方式发布。
- 缓存发布、统计和回收统一走 `yali_core::cache`，遵守跨进程租约和活动锁，不跟随符号链接清理。
- NSPasteboard 只在主线程访问；剪贴板去重同时使用 changeCount 和内容摘要。
- watch 必须忽略自身产物，并按 settleMs 与 `(mtime, size)` 去重，避免自激。
- 调用 C、Vision 或 vips API 前先查本机 SDK 头文件或做最小实测，不凭记忆编造接口。
- workflow 安装会修改用户 Services。模板变更后的安装、公开 API 刷新和卸载都要在任务卡中记录，并在执行前确认用户授权。
- macOS 仅按 Command Line Tools 约束开发，禁止 `xcodebuild`、`swift test` 和 SwiftUI 宏。Swift 只用于明确批准的 `helpers/` 小工具。
- UI 改动必须有任务卡或用户明确范围，不借文档任务顺手改样式。

## 验证命令

| 修改范围 | 必跑命令 |
| --- | --- |
| `yali-core` | `cargo test --locked -p yali-core` |
| Tauri 壳层 | `cargo test --locked -p yali-app` |
| CLI | `cargo test --locked -p yali-cli` |
| UI | `pnpm --dir ui test`、`pnpm --dir ui build` |
| 完整 App | `bash scripts/build.sh` |
| 发布脚本 | `python3 -m unittest discover -s scripts -p 'test_*.py' -v` |
| 压缩质量 | 重建 CLI 后运行 `benchkit/bench.sh` |

按改动范围运行必要检查，报告实际命令和退出结果。金样只按体积比和 SSIM 劣化超过 3% 判失败，耗时、内存和 CPU 只作为观察项。浏览器演示、自动化脚本和截图不能替代用户对 Finder、通知、托盘、剪贴板、watch、窗口手感和产物兼容性的人工验收。

## 危险操作

删除文件或目录、批量移动、修改用户或系统配置、安装或卸载 Services、数据库结构变更、批量数据更新、发送敏感数据、调用生产 API、全局安装或更新核心依赖、`git commit`、`git push`、`git reset --hard`、强制推送和远端历史改写，都要在执行前说明类型、影响范围和风险，并取得明确的“是 / 确认 / 继续”。

## 交付要求

交付说明至少包含：改动文件、设计原则如何应用、验证命令及结果、当前未完成的人工验收、已知限制和回退方式。不要把自动化通过写成用户已经亲验，也不要把未验证的平台写成已支持。
