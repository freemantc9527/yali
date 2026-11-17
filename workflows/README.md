# workflows · Finder 右键「鸭梨 · 压一下」

M0.5 主通道：用户级 `.workflow`（免 Xcode 免签名）。Finder 里选中图片 → 右键 →
快速操作 →「鸭梨 · 压一下」→ 后台压缩 → **一条**聚合通知。

## 安装 / 卸载

```bash
workflows/install-workflows.sh                # 自动找 yali（$YALI_BIN → PATH → target/{release,debug}）
workflows/install-workflows.sh --uninstall    # 卸载
```

- 二进制建议用 release：`cargo build --release -p yali-cli`（debug 冷启动明显慢）
- 装到 `~/Library/Services/鸭梨 · 压一下.workflow`，安装后 flush pbs 刷新菜单
- 模板在 `templates/yali-quick-action.workflow/`，安装时把 `__YALI_BIN__` 渲染成绝对路径

## 人工验收（Agent 无法自验的部分）

- [ ] Finder 右键菜单里出现「鸭梨 · 压一下」（若没出现：系统设置 → 键盘 → 键盘快捷键 → 服务 里勾选）
- [ ] 多选 3 张只弹一条通知，文案含「省了多少」
- [ ] 通知点击焦点行为（M2 再优化）

已自动验证：`automator -i <jpg> <workflow>` 全链路（产出 `-{profile}` 文件 + JSON 摘要 + 通知进程成功）；
pbs 注册表可见 `NSBundleIdentifier = app.yali.quick-action`；CLI `--notify` 聚合文案单测三态覆盖。

## 实现要点（避坑）

- Services 的 PATH 只有 `/usr/bin:/bin`：workflow 脚本显式 `export PATH`，CLI 引擎解析自带
  Homebrew 目录与 `brew --prefix` 绝对路径兜底（坑账本）
- 通知由 CLI 侧 `--notify` 聚合发出（多选一条），文案规则：`N 张省了 X，M 张压不动已保持原样，K 张失败`
- 负优化拦截生效时不落盘，仍如实计入通知
- 撤销：`yali undo <目录>` 按 `.yali-undo.json` 还原（右键批次的清单写在图片所在目录）
