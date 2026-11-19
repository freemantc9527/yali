# Finder 快速操作

用户级 `.workflow`，免 Xcode / 签名。选图 → 右键快速操作 → 静默处理 → 一条聚合通知；精调归 GUI / CLI。当前两条，产品约束最多三条。

| 菜单项 | 命令 | 适用 |
|---|---|---|
| 鸭梨 · 压一下 | `compress --profile default` | 自动路由压缩 |
| 鸭梨 · 转 JPG | `compress --profile jpg` | 转 JPEG；JPEG 输入隐藏此动作 |

## 安装与卸载

安装脚本会写入用户 Services、pbs 偏好并刷新 Finder，执行前按当前用户授权确认范围。

```bash
cargo build --release -p yali-cli
workflows/install-workflows.sh               # 安装两条动作
workflows/install-workflows.sh --uninstall   # 卸载并清历史 bid 的 pbs 项
```

CLI 查找顺序：YALI_BIN → PATH → target/release、target/debug。模板位于 `templates/yali-quick-action.workflow/`，安装时将占位替换为绝对路径，目标为 `~/Library/Services/`。

## 必须保留的系统约束

- 光复制 workflow、flush pbs 不够，需登记 NSServicesStatus 与 FinderOrdering；后者必须是 int，用 defaults export/import + plistlib 保类型，字符串会让 Finder 丢失整个快速操作菜单。
- 每个动作声明具体 UTI；只用 public.image 在普通目录不匹配。「转 JPG」不声明 public.jpeg。
- Services PATH 仅 /usr/bin:/bin；CLI 自带 Homebrew 固定目录兜底，不在运行时调用可能联网阻塞的 brew --prefix。
- 出厂档按 `.seed` 签名补种，不复活用户主动删除的预设。
- 通知聚合成功、未减小与失败；未减小不落盘。`yali undo <目录>` 读取 `~/.yali/data/undo/` 中对应目录的撤销记录，兼容旧版同目录 `.yali-undo.json`。

## 验收状态

2026-11-17 已自动验证 automator 真实产物、pbs 注册及 CLI 通知三态；用户已确认 Finder 两条动作、JPEG 隐藏转 JPG、多选一条聚合通知。历史实现与修复见 [T-006](../docs/tasks/T-006-m0.5-workflow.md)、[T-012](../docs/tasks/T-012-quick-action-set.md)。

- [ ] 通知点击后的焦点行为仍待人工验收。
