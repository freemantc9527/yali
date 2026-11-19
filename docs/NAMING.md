# NAMING · 鸭梨 / Yali 命名注册表（唯一权威）

> 本表是项目所有名称的权威来源，其他文档冲突时以本表为准。命名变更须走 ADR（见 `docs/decisions/README.md`），阈值：凡是出现在用户可见处、包名、路径、env 前缀的名称变动，都要留一份 ADR。

## 1. 品牌层

| 项 | 定名 | 说明 |
|---|---|---|
| 中文产品名 | **鸭梨** | 谐音"压力"反梗立意 |
| 英文产品名 | **Yali** | 品牌语境大写；技术语境一律小写 `yali` |
| 中文 tagline | **让图片没有压力** | |
| 英文 tagline | **No pressure for your pictures.** | |
| 场景文案 | 「右键压一下，图片没鸭梨」 | |
| 菜单动词 | **压一下**（en: Compress） | Finder 右键主项 |
| 吉祥物 | 抱梨的鸭（原创绘制） | 名字留白，开源后社区征名 |

## 2. 分发与标识层

| 产出物 | 定名 |
|---|---|
| GitHub | `freemantc9527/yali` |
| App 包 | `Yali.app`（productName=`Yali`，ASCII 保工具链兼容） |
| 显示名本地化 | zh-Hans `CFBundleDisplayName=鸭梨`；en=`Yali` |
| bundle identifier | `app.yali`（所有数据路径派生源） |
| brew tap / formula（规划） | `yali-app/tap` / `yali` |
| License | Apache-2.0 |
| 版本 | SemVer，首发 v0.1.0；Keep a Changelog |

## 3. 代码与二进制层

| 产出物 | 定名 | 规则 |
|---|---|---|
| CLI 安装二进制 | **`yali`** | 用户可见唯一命令名；文档示例一律写 `yali` |
| CLI crate | `yali-cli` | cargo 包名带 -cli，产物是 `yali` |
| 核心 crate | `yali-core` | |
| 平台集成 crate | `yali-integration` | 预注册，尚未创建；平台差异 trait 收口 |
| Vision 封装 crate | `yali-vision` | M3 起可选 |
| 前端包 | `yali-ui` | ui/，pnpm 私有包不发布 |
| Vision helper | `yali-vision-helper` | CLT swiftc 编译；子命令 cut/ocr/saliency |
| Tauri 壳 | `src-tauri/`（package `yali-app`） | |

## 4. 系统集成物

| 项 | 定名（zh / en） |
|---|---|
| workflow 两项 | `鸭梨 · 压一下` / Compress · `鸭梨 · 转 JPG` / Convert to JPG |
| workflow 标识 | `app.yali.quick-action.compress` · `app.yali.quick-action.to-jpg`（模板与安装脚本为准） |
| 菜单栏图标 | `yaliTemplate.png`（@2x，模板图自动明暗） |
| 通知标题 | 鸭梨 / Yali |

## 5. 数据与路径层

| 项 | 值 |
|---|---|
| 应用数据根 | `~/.yali/`，`YALI_HOME` 可重定向 |
| Profile / 设置 | `…/profiles/*.json` · `…/settings.json` |
| 指标 / 日志（规划） | `…/metrics.sqlite` · `…/logs/` |
| 撤销清单 | `~/.yali/data/undo/`，兼容旧旁置 `.yali-undo.json` |
| 包内 CLI | `Yali.app/Contents/MacOS/yali` |
| 稳定 CLI 入口 | `~/.yali/bin/yali`，指向已安装 App 内 CLI，见 ADR-006 |
| Finder workflow | `~/Library/Services/鸭梨 · 压一下.workflow` / `鸭梨 · 转 JPG.workflow` |
| 环境变量 | `YALI_PROFILE` · `YALI_ENGINES_PATH` · `YALI_HOME` |

## 6. 书写规则

品牌语境「鸭梨/Yali」，技术语境一律小写 `yali`；禁止 `YALI`（env 前缀除外）/`YaLi`；中英混排首现「鸭梨（Yali）」。
