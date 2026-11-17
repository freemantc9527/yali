# BENCHMARKS · 引擎选型实测数据

> 2026-11-16 于 Apple M1 Pro / macOS 27.2 实测。素材：8 张真实照片（1500×2000 级，JPEG 合计 4.5MB）+ PNG/GIF/HEIC 派生样本。度量：`magick compare -metric SSIM`（vs 原图相似度）。完整方法学与逐文件数据见私有档案；本文件为选型结论态。

## 1. JPEG 重编码（同档画质：moz q75 ≈ webp q75 ≈ avif q50 ≈ jxl q75）

| 编码器 | 相对原图总大小 | 平均 SSIM | 单图均耗时 | 选型定位 |
|---|---|---|---|---|
| mozjpeg q75 | **-58.3%** | 0.985 | 165ms | JPEG 输出主力 |
| WebP q75 | -68.8% | 0.982 | 215ms | 网页预设 |
| AVIF q50 | **-78.3%** | 0.981 | 168ms | 体积最优，网页/归档预设 |
| JXL q75 | -65.0% | — | 446ms | 生态回暖（Chrome/FF 已原生），观察位 |
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

结论：有损 PNG 管道必须带 oxipng 后处理；UI 截图类走无损 WebP（不糊边，实测更优，M3 分类器路由）。

## 3. GIF / 动图（3 帧 970KB 样本）

| 方案 | 相对源 |
|---|---|
| gifsicle -O3 --lossy=80 | -2.7%（格式天花板） |
| **gif2webp q60** | **-24.4%** |

结论：动图收益靠换容器（Animated WebP 默认策略 / Animated AVIF 更优待测）；gifsicle 保留兼容输出。

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
