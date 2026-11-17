# Yali 引擎依赖（版本以 2026-11-16 本机实测为基准）
# 用法: brew bundle --no-lock（sha256 锁定在 M0 由 CI 生成 Brewfile.lock.json）
brew "mozjpeg"        # JPEG 有损主力（keg-only，cjpeg 路径注意 keg prefix）
brew "jpeg-turbo"     # 备用/对照
brew "webp"           # cwebp/dwebp/gif2webp
brew "jpeg-xl"        # cjxl/djxl（无损归档 + JXL 输出）
brew "libavif"        # avifenc
brew "aom"            # AVIF 质量档编码器
brew "svt-av1"        # AVIF 速度档编码器
brew "pngquant"       # PNG 有损量化
brew "oxipng"         # PNG 无损优化 + pngquant 后处理
brew "gifsicle"       # GIF（GPL，子进程调用，不链接）
brew "gifski"         # GIF 高质量编码（Rust）
brew "vips"           # 缩放/转码主力（LGPL，子进程/动态）
brew "imagemagick"    # 仅 benchkit 度量用（SSIM），不进产品分发

# 开发工具（不进 bundle，开发机自备）：rust 工具链、node、pnpm（前端包管理器，见 docs/decisions/ADR-004-pnpm.md）
