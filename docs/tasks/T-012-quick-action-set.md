# T-012 · 右键动作三件套（分享场景取舍）

goal: 把右键收敛成两条服务分享场景的动作（压一下 / 转 JPG），并修掉这条路上暴露的质量 bug
context: docs/DESIGN.md §4 入口矩阵；用户诉求「最常用的就两个：压缩图片、转成 jpg，用于分享」；强迫症约束：快速操作里不超过 3 项
decision:
  - 鸭梨 · 压一下 → `compress --profile default`（保持原格式，减体积）
  - 鸭梨 · 转 JPG → `compress --profile jpg`（新增出厂档，兼容性最好）
  - WebP/AVIF/HEIC/JXL 归档**不进右键**，归 GUI 与 CLI（渐进披露：右键永不询问）
constraints:
  - 模板参数化（`__TITLE__ / __BID__ / __COMMAND__ / __SEND_TYPES__`），安装脚本循环生成
  - pbs 登记必须保类型（FinderOrdering 值必须 int）；卸载要清历史 bid
  - 出厂档新增必须能被老目录补种（`.seed` 签名机制）
acceptance:
  - `automator -i <file> <workflow>` 三个动作各自产出正确文件
  - RGBA PNG → JPG 白底正确（PSNR 45dB vs 白底参考），体积不劣于 mozjpeg 基线
  - GIF → JPG 不再被缩成 128×96（尺寸守卫实测拦住过）
  - `bench.sh --profile jpg` 三指标出表；default/web 基线仍绿
  - cargo 46 + ui 48 全绿（T-012 顺带把 Tauri 冒烟的档案列表期望从五档改六档）
estimated: 3h
manual-check: true（Finder 快速操作里两条鸭梨；JPEG 文件右键不出现「转 JPG」）

## 过程中修掉的两个真 bug
1. **cjpeg / libvips 把 RGBA PNG 的 alpha 合成到黑底**（实测 PSNR 2.2dB，整图发暗）
   → PNG→JPEG 前先用 `vips flatten --background 65535` 无损压平到白底（libvips 的
   background 用 16 位标度，255 无效），再交 mozjpeg。sips 虽正确但体积大 2.6 倍，只作回落。
2. **vipsthumbnail 不带 `--size` 时默认输出 128×128**（GIF→JPG 被缩成 128×96）
   → 兜底转换改用 `vips copy`（全尺寸）；预缩放路径仍用 vipsthumbnail 但显式传 --size。
   这条是尺寸守卫第一次在真实用例上拦住数据损坏，证明守卫必要。

## 用户实测反馈修正（第二轮）
1. 「压到 500K」下线（有前两条足够）；安装脚本负责清历史 bundle + pbs 残留
2. 「转 JPG」按 UTI 精确声明，不声明 `public.jpeg` → JPEG 文件右键看不到这条
3. **PNG 转 JPG 变「底片」**：根因是无条件 `vips flatten`——libvips 对**无 alpha 的
   Type:Palette PNG** 做 flatten 会输出 graya 灰白图（实测 PSNR 0.6）。修法三层：
   读 IHDR color type（调色板再看 tRNS）确认真有透明 → 才 flatten；flatten 产物
   color type 必须为 2，否则判失败回落 sips；benchkit 新增 `palette.png` 样张 +
   基线按档位分文件（`baseline-jpg.json`），这条路径从此有回归守护
4. 附带修：pngquant 对已量化 PNG 退出码 99 → 调色板 PNG→PNG 强制 oxipng 无损（-o 1 快速档）
