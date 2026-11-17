# benchkit · 合成金样回归

质量相关变更（参数映射 / 编码器 / 管道）**合并前必须过 `bench.sh`**。本目录不进 cargo 测试回环，
避免拖慢 `<60s` 单测预算。

## 一键使用

```bash
benchkit/generate.sh                 # 生成合成金样（确定性的，可复跑；产物入库）
benchkit/bench.sh                    # 默认档（default）三指标回归
benchkit/bench.sh --profile web      # 换档
benchkit/bench.sh --format webp      # 临时覆盖输出格式（不动磁盘档案）
benchkit/bench.sh --update-baseline  # 有意的质量变更后重写基线
```

- 输出表格：每样 × 引擎的 体积比 / SSIM / 耗时中位数（3 轮取中位，单图口径）
- 与 `baseline.json` 比对：体积比↑ / SSIM↓ / 耗时↑ 任一劣化 >3% → 退出码非零
- 质量门（DESIGN §9）：level≥5 时**单代照片类样本** SSIM≥0.95

## 样张（全部合成，合规红线）

| 样张 | 内容 | 说明 |
|---|---|---|
| `photo.jpg` | 渐变 + 结构纹理 + 中等噪声，1920×1280 q92 | 照片类代表；`-seed 42` 固定随机，可复跑 |
| `ui.png` | 纯色块 + 文字模拟，1600×1000 | UI 截图类 |
| `anim.gif` | 12 帧分形噪点，480×360 | 动图类 |
| `transparent.png` | 圆形 + 半透明块 + 文字，alpha | 透明 PNG 类 |
| `photo_heic.heic` | sips 编自 `photo.jpg` | HEIC 类（macOS 硬编） |

**禁止个人照片衍生物入库**；个人素材放 `benchkit/local-samples/`（已 gitignore）。

## 度量口径（与任务卡的偏离记录，均经实测）

1. **SSIM 用 ffmpeg 而非 `magick compare -metric SSIM`**：本机 ImageMagick 7.1.2-36 构建的
   SSIM 语义损坏（同像素输入输出 `0` 而非 `1`，翻转图 0.279），不可用作回归信号。
   ffmpeg 的 `ssim` 滤镜语义正确（同图=1.0），且在 jpeg/png/webp/gif/avif 上可直解。
2. **含 alpha 的图先压平到白底再比**：直接比 RGBA 会把 alpha 通道差异计入，实测一个
   PSNR 51.8dB（视觉无损）的透明件 SSIM 只有 0.33，属伪影。
3. **`vips flatten` 不能对无 alpha 的 PNG 调用**：会把 Type:Palette 图输出成 graya 灰白图
   （实测 PSNR 0.6，整图报废）。管道改为先读 IHDR color type / tRNS，只有真带透明才压平。
   `palette.png` 样张就是为这条而加。同理 pngquant 对已量化的调色板 PNG 会退出码 99
   （达不到最低质量），管道强制这类输入走 oxipng 无损优化。
4. **绝对质量门只对单代照片类生效**：平涂图形/动图的 SSIM 对边缘 ringing 极敏感
   （q75 webp 在透明图形上仅 0.86~0.92，而视觉与 PSNR 均正常）；HEIC 样张源本身有损、
   再编码属二代生成，绝对值无意义。这两类由 baseline 的 3% 漂移比守护。
5. **HEIC 输入在管道内用 sips 解码**（不走 vips）：libvips 的 heif loader 会返回内嵌
   缩略图（1920×1280 只解出 128×85），管道另有输出尺寸守卫兜底。

## 基线策略

- `baseline.json` 由首次全量跑生成并入库；`--profile` 不同的运行不与基线比对（档位不同不可比）
- 只在**有意的质量变更**后用 `--update-baseline` 重写，并在 commit message 里说明原因
- 计时指标在 16GB M1 Pro 上取单图中位数；机器负载波动 >3% 属正常误报，重跑一次确认
