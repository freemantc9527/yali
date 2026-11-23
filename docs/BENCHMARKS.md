# BENCHMARKS · 引擎选型实测数据

> 2026-11-16 于 Apple M1 Pro / macOS 27.2 实测。素材：8 张真实照片（1500×2000 级，JPEG 合计 4.5MB）+ PNG/GIF/HEIC 派生样本。当时的 SSIM 使用 `magick compare -metric SSIM`，仅保留为历史选型记录；该构建后来发现同图结果异常，现行回归以 [benchkit](../benchkit/README.md) 的 ffmpeg 口径为准。

## 1. JPEG 重编码（同档画质：moz q75 ≈ webp q75 ≈ avif q50 ≈ jxl q75）

| 编码器 | 相对原图总大小 | 平均 SSIM | 单图均耗时 | 选型定位 |
|---|---|---|---|---|
| mozjpeg q75 | **-58.3%** | 0.985 | 165ms | JPEG 输出主力 |
| WebP q75 | -68.8% | 0.982 | 215ms | 网页预设 |
| AVIF q50 | **-78.3%** | 0.981 | 168ms | 体积最优，网页/归档预设 |
| JXL q75 | -65.0% | 未测 | 446ms | 生态回暖（Chrome/FF 已原生），观察位 |
| jpegtran 无损 | -5.7% | 1.0000 | ~50ms | 零损失白捡 |
| **JXL 无损包裹** | **-17.2%** | 1.0000 | ~58ms | ⭐ 归档模式：djxl 逐比特还原 |

口径注意：源已是 JPEG（"二压"，即应用真实场景）；跨格式 quality 标尺不同，体积比带 ±5% 容错。

## 2. PNG（2.39MB 源）

| 方案 | 相对源 | 耗时 |
|---|---|---|
| pngquant 中档（quality 70-100） | -45.2% | 771ms |
| pngquant 高档（35-65） | -67.2% | 1.35s |
| **pngquant + oxipng --opt max 后处理** | **-50.9%**（比单跑再省 10.3%） | 3.8s |
| oxipng 纯无损 | -10.8% | 2.6s |

结论：当时建议有损 PNG 加 oxipng 后处理；无损 WebP 是截图候选方案，分类路由属 M3。

## 3. GIF / 动图（3 帧 970KB 样本）

| 方案 | 相对源 |
|---|---|
| gifsicle -O3 --lossy=80 | -2.7%（格式天花板） |
| **gif2webp q60** | **-24.4%** |

结论：动图收益靠换容器（Animated WebP 可选，Animated AVIF 待测）；gifsicle 保留兼容输出。

## 4. HEIC / 缩放

| 项 | 结果 | 结论 |
|---|---|---|
| sips HEIC q70（系统 ImageIO 硬编） | **-42.3%，112ms** | HEIC 输入输出直接吃系统能力，零捆绑 |
| heif-enc（x265 软编） | -9.1%，1.14s | 不用 |
| vips thumbnail（热） | 100-164ms | 缩放主力（lanczos3+gamma 正确） |
| sips -Z | 94ms | 快但缩放算法弱，仅兜底 |
| ImageMagick lanczos | 162ms | benchkit 度量用，不进产品 |

## 5. 系统能力实证（影响架构决策）

- macOS 27 ImageIO **原生解码 JPEG XL**（sips 转码成功）→ JXL 归档文件在本机生态无摩擦，应用只捆绑编码器
- Vision `VNGenerateForegroundInstanceMaskRequest`（macOS 14+）CLT 可编译可运行 → 抠图能力零成本本地化
- 工具链完整边界见 TOOLCHAIN.md

## 6. 待补测（benchkit 自动化时纳入）

jpegli（google/jpegli，需 cmake 自建）· Animated AVIF 体积 · gifski 质量向 · UI 截图素材集（验证分类器假设）· 目标大小二分收敛实测

## 7. 冒烟手测备注（2026-11-17；正式回归已由 benchkit 接管，见 §8）

样张：plasma 分形噪声 JPEG（噪声内容对压缩器不友好，省率偏高属正常）；机器 M1 Pro。

| 档 | 引擎 | 结果 | 耗时 |
|---|---|---|---|
| default (level 4) | mozjpeg cjpeg q60 | 726KB → 85KB（-88%） | ~95ms，p50 稳定 |
| mail (targetBytes 500KB) | mozjpeg 二分（floor 3 生效） | 726KB → 159KB | 725ms（≤5 轮 + vips 预缩放） |
| web (level 5) | cwebp q75 | 726KB → 97KB（-87%） | 136ms |
| lossless 归档 | cjxl --lossless_jpeg=1 | 726KB → 587KB（-19%） | 63ms（与 §基准 -17.2% 同量级） |

行为验证：负优化拦截（预压 q30 样张再压 → status larger_output，不落盘）✓；undo 100% 还原 ✓；doctor 引擎路径全对（cjpeg 走 /opt/homebrew/opt/mozjpeg keg，PATH 李鬼未踩）✓。

工程坑新增：**vipsthumbnail 不认 dotfile 输出路径，会把图片写进 stdout**，预缩放中转文件必须用非隐藏名（已修，代码注释有记）。

## 8. benchkit 自动化基线（2026-11-17 起）

三指标回归已脚本化：`benchkit/bench.sh`（体积比 / SSIM / 耗时中位数），基线按档位分文件（`benchkit/baseline-default.json` / `baseline-jpg.json`），
体积比 / SSIM 任一劣化 >3% 退出码非零；耗时仅观察，不阻塞验收（用户确认，2026-11-18）。default 档首跑（photo 531KB → 49KB / SSIM 0.965 / 121ms；ui 34.8KB → 4.7KB / 1.000 / 74ms；
gif 966KB → 661KB / 0.823 / 184ms；transparent 33KB → 5.3KB / 0.932 / 66ms；heic 264KB → 258KB / 0.989 / 169ms）。
M0 出口另记历史 p50 119ms、level≥5 单代照片 SSIM 0.9657，undo 逐字节还原。耗时不作验收。

## 9. 转 JPG 档实测（2026-11-17，bench --profile jpg）

| 样张 | 引擎 | 体积比 | SSIM | 耗时中位 |
|---|---|---|---|---|
| photo.jpg | mozjpeg | 0.093 | 0.965 | 95ms |
| ui.png（不透明） | mozjpeg | 0.395 | 0.995 | 188ms |
| transparent.png（RGBA） | flatten+mozjpeg | 0.356 | 0.913 | 154ms |
| photo_heic.heic | sips 解码+mozjpeg | 0.090 | 0.534（PNG 中转测量偏保守；对原图 ffmpeg 直测 0.80） | 696ms |

RGBA→JPG 必须先压平 alpha：cjpeg/libvips 直吃 RGBA 会合成到黑底（PSNR 2.2dB）；
`vips flatten --background 65535` 后走 mozjpeg，比 sips 直出小 2.6 倍。
