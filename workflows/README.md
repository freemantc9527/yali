# Finder 快速操作

用户级 `.workflow`，由构建后的 App 管理；接收者不需要源码、Homebrew 或 Python。

| 菜单项 | 命令 | 适用 |
|---|---|---|
| 鸭梨 · 压一下 | `compress --profile default` | 自动路由压缩 |
| 鸭梨 · 转 JPG | `compress --profile jpg` | 转 JPEG；JPEG 输入隐藏此动作 |

## 安装与卸载

先将完整 `Yali.app` 移入 `/Applications/` 或 `~/Applications/`，再到 **设置 → 自动化 → Finder 右键操作** 安装 / 修复 / 卸载。操作立即执行，与设置保存独立。App 位于下载隔离目录或挂载镜像中时拒绝登记。

两个 workflow 位于 `~/Library/Services/`，都调用 `~/.yali/bin/yali`，该链接指向 App 内的 `Contents/MacOS/yali`。`YALI_HOME` 重定向时链接跟随数据根；Services 仍属于当前 HOME。移动 App 后重新打开一次可修复已有链接；App 丢失时动作提示重新打开或安装。覆盖升级沿用原路径。不会修改 shell 的 PATH。

开发者可以指定 App 内嵌 CLI 调用同一套逻辑：

```bash
YALI_BIN="/Applications/Yali.app/Contents/MacOS/yali" bash workflows/install-workflows.sh
YALI_BIN="/Applications/Yali.app/Contents/MacOS/yali" bash workflows/install-workflows.sh --uninstall
```

CLI 提供 `yali services status|install|uninstall`，返回实际文件与链接状态。状态不冒充系统启用状态。卸载只移除属于鸭梨的两条 workflow，保留图片、预设、配置与 CLI 链接。同名非托管文件和非托管 CLI 入口不覆盖。

## 系统约束

- 使用本机 SDK 声明的公开 `NSUpdateDynamicServices()` 刷新，不改写 `pbs` 内部的 `NSServicesStatus` / `FinderOrdering`，不 kill Finder 或 pbs。系统若未显示菜单，由用户到系统设置的“服务”或“扩展”中启用。此策略按 T-023 / ADR-006 取代旧安装脚本的内部偏好写入。
- 每个动作声明产品支持的十类位图具体 UTI；不用抽象 public.image。“转 JPG”不声明 public.jpeg。未分发 Node/SVGO，不在右键中声明 SVG。
- workflow 模板与生成器在编译期进入核心，运行时不读源码树。`"$@"` 保留多选文件边界；shell 引号与 XML 转义独立处理。
- 安装操作加文件锁串行，先预检与暂存，再发布；发布失败恢复旧 workflow 与 CLI 链接。只清理本轮管理的暂存路径。
- 引擎只从包内解析，sips 使用系统绝对路径。App 内 libvips 插件与动态依赖一并携带，不依赖 Services 的 PATH。
- 多选通知聚合成功、未减小与失败；未减小不落盘。CLI 调用同一 App 的短时模式发送 macOS 原生通知，使用鸭梨图标，不创建工作台；首次需允许系统通知，点击沿用系统默认行为，无业务回调。发送失败不影响压缩；开发态裸 CLI 不回落脚本通知。撤销记录统一由 core 数据根管理。

## 验收

T-023 的隔离脚本验证真实 Automator workflow 产物与安装 / 重装 / 卸载，具体结果见任务卡。Finder 菜单启用、可见性和通知交互必须原生人工验收，自动化不替代用户亲验。
