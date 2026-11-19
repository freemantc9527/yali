# T-026 · 鸭梨原生完成通知

status: 已实施并安装，待人工验收（2026-11-19）
goal: Finder 压缩与剪贴板完成通知使用鸭梨图标，不再由系统脚本程序发送
manual-check: true

## 已批准范围

- 用户确认改用鸭梨原生通知，仅提示完成；不实现点击回调、自定义按钮或 Finder 定位。
- CLI 调用完整 App 的短时通知模式，使用 App 自身身份；不创建工作台窗口，发完退出。
- 剪贴板复用相同通知实现；保留 Finder 多选只发一条汇总。
- 重建完整 App，更新本机鸭梨的两条 Finder 快速操作及稳定 CLI 链接。
- 不改压缩算法、UI 样式，不创建分支或 push；用户后续授权审查通过后提交到当前分支，并将旧 `benchkit/samples/.yali-undo.json` 的删除纳入提交。

## 根因

CLI `notify` 和剪贴板 `notify` 均调用 `osascript display notification`，标题「鸭梨」不改变系统认定的发送者。通知显示脚本图标，系统默认“显示”也打开脚本程序。本机旧 workflow 仍调用项目 `target/release/yali`，需同步切换至 App 内 CLI。

## 验收计划

- core / CLI / app 测试与完整 `bash scripts/build.sh`。
- 真 App 内原生通知权限、投递、图标和不创建工作台窗口；不实现点击业务动作。
- 真实 workflow 单图 / 多图汇总，产物正常且 CLI JSON 不被通知子进程日志污染。
- 更新安装后核对两条 workflow 和 CLI 链接，公开 API 刷新 Services。
- 未经用户亲验，图标与通知手感继续标记待人工验收。

## 实现与原则

- `yali_core::runtime::notify` 统一检查通知开关、定位当前 bundle 的 `yali-app`，以独立参数调用 `--yali-notify`。子进程输出被捕获，失败只记录到 stderr，不改变压缩结果。
- App 主程序在启动 Tauri 前识别通知模式，通过本机 SDK 已核对的 `UNUserNotificationCenter` 请求授权并投递；主线程运行循环等待系统完成，最迟 60 秒超时。短时进程使用禁止激活策略，不创建工作台、托盘或监听器；无自定义通知 category、按钮或点击 delegate。
- 首次需用户允许系统通知；不替用户修改系统权限。开发态裸 CLI 不回落脚本通知，而是保留压缩结果并提示使用完整 App。
- CLI 与剪贴板删除了两处 `display notification`，共用同一原生发送路径。剩余 `osascript display alert` 仅用于 workflow 找不到 CLI 时的错误对话框，属于不同用途，保留。
- KISS / DRY：使用已有 App 的短时模式，复用单一发送入口，不增加常驻服务、IPC 队列、深链协议、Swift helper 或业务点击行为。新增依赖均为现有锁文件中的 objc2 / block2 版本，无核心依赖升级。

## 实际验证

```text
cargo test --locked --offline -p yali-core -p yali-cli -p yali-app -- --test-threads=1
95 passed（9 suites，含新增 2 项通知模式集成测试与 1 项 CLI 发送回归测试）。

bash scripts/build.sh
真实 UI、release App 构建通过；111 个 Mach-O 依赖及 ad-hoc 签名校验通过；10 类图片 / 10 个具体 UTI 通过。

~/Applications/Yali.app/Contents/MacOS/yali services install
installed=true，needsRepair=false；公开 NSUpdateDynamicServices 刷新成功。
~/.yali/bin/yali -> ~/Applications/Yali.app/Contents/MacOS/yali
两条 workflow 均已移除项目 target/release/yali 入口。

automator -i - ~/Library/Services/<鸭梨动作>.workflow
单 JPEG、三图 default、透明 PNG 转 JPG：三次 exit=0、stderr 为空，五个产物生成，原图 SHA-256 未改变。
单图 JPEG：531019 -> 49178 bytes。
三图：photo=49178，ui=4696，transparent=5276 bytes。
转 JPG：transparent-jpg.jpg=11724 bytes。

系统日志（2026-11-19 16:21:55–16:21:56）：
Connection app.yali with path: /Users/taocheng/Applications/Yali.app
三次 workflow 恰好 3 条 Adding new request / 3 条 Presenting ... as banner。
```

原生仅通知模式首次授权后 exit=0、stdout / stderr 为空；剪贴板标题及含引号、反斜杠的正文同样投递成功。两项集成测试验证参数不全、裸程序缺少 bundle 身份时以正常错误退出，不误启动工作台或触发 UserNotifications 的身份异常。

临时数据根下分别实测关闭通知、裸 CLI 发送失败：均成功压缩，stdout 恰好一条合法 JSON；关闭时 stderr 为空，发送失败时 stderr 提示使用完整 App。未改压缩算法，不重跑金样或修改基线。

本轮原生验证使用系统投递日志和真实 Automator，未操作用户剪贴板触发复制事件，也未将自动化推定为人工验收。首轮测试时用户前台应用发生变化，因此未用前后台应用名称对比作“不抢焦点”的严格证明；当前实现不创建窗口、禁止激活，最终感受待用户亲验。

- [ ] 用户确认通知图标与外观、通知发送不打断当前操作。
- [ ] 用户确认 Finder 两条菜单实际使用，以及真实复制图片后的剪贴板通知。
- 点击沿用系统默认行为，无需验收业务回调（用户已明确取消）。

## 提交前复验（2026-11-19）

- 以 `6209ed1a59653b8ea73419c74788332ddcd9aa49` 导出基线与候选副本，仅向候选应用本次变更，各用独立构建目录。新增 CLI 集成测试将真实 CLI 放入带空格的临时 bundle，以替身 App 记录通知参数并输出噪声，验证失败件也通过同包 App 发一条通知、子进程噪声不进入 CLI stdout。
- 旧基线运行该测试：1 failed（通知 App 未被调用，回执不存在）；沙箱明确禁止启动旧 osascript，不发送真实旧通知。候选在临时数据根串行运行完整 Rust 测试：95 passed。初次并发隔离运行有一项既有 TIFF 测试失败；独立 TIFF 复验与串行完整复验通过。既有命令测试会修改进程级 `YALI_HOME`，本次未扩展到测试环境重构。
- 再次 `bash scripts/build.sh` 通过；构建与已安装 App 的主程序、CLI、图标和 Info.plist SHA-256 一致，`services status` 为 installed=true / needsRepair=false。原生投递沿用上轮相同二进制的三次 workflow 日志证据，本次未再次发送测试横幅。
- 两处脚本完成通知均已替换；缺少 CLI 的 `display alert` 错误对话框保留。删除的旧撤销清单无运行时直接引用；用户已明确将删除纳入本次提交。
