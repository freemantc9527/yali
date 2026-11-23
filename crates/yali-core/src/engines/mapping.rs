//! level→参数映射 v1（出厂标定表）。权威公式：docs/DESIGN.md §3。
//!
//! JPEG `quality = 100 - 10×level`；WebP/AVIF `quality = 100 - 5×level`；
//! GIF `colors = round((10-L)/9×223)+32`；PNG level 1-2 走 oxipng 无损、3-10 走
//! pngquant speed/quality 线性插值（M3 换 SSIM 目标点标定：{0.99…0.90} 十档）。
//! 本模块是纯函数，单测钉死公式；引擎参数拼装见 encoders.rs。

/// JPEG（mozjpeg cjpeg）quality = 100 − 10×level
pub fn jpeg_quality(level: u8) -> u8 {
    100 - 10 * level
}

/// WebP（cwebp）/ AVIF（avifenc）quality = 100 − 5×level
pub fn webp_quality(level: u8) -> u8 {
    100 - 5 * level
}

/// GIF（gifsicle）colors = round((10−L)/9×223)+32：L=1 → 255 色，L=10 → 32 色
pub fn gif_colors(level: u8) -> u32 {
    let l = level.clamp(1, 10) as f32;
    (((10.0 - l) / 9.0) * 223.0).round() as u32 + 32
}

/// PNG 有损档（pngquant）quality 下界：100 − 8×(L−2)，夹到 [30, 99]
pub fn pngquant_quality_min(level: u8) -> u8 {
    (100u8.saturating_sub(8 * (level.saturating_sub(2)))).clamp(30, 99)
}

/// PNG 有损档（pngquant）quality 上界：100 − 4×(L−2)，夹到 [40, 99]，恒 ≥ 下界
pub fn pngquant_quality_max(level: u8) -> u8 {
    let max = (100u8.saturating_sub(4 * (level.saturating_sub(2)))).clamp(40, 99);
    max.max(pngquant_quality_min(level))
}

/// PNG 有损档（pngquant）speed：1.5×(L−2) 四舍五入，夹到 [1, 11]
pub fn pngquant_speed(level: u8) -> u8 {
    let raw = ((level.saturating_sub(2) as f32) * 1.5).round() as i32;
    raw.clamp(1, 11) as u8
}

/// PNG 无损优化级别（oxipng -o）：
/// - 无损档 / level 1-2：深度优化（-o 4，慢但省得多）
/// - level ≥3 却落到 oxipng（例如已经是调色板的 PNG，不做二次量化）：
///   走快速档 -o 1 —— 实测 900KB 调色板样张 -o 1 是 353ms/800966B，
///   -o 4 是 1100ms/800276B，多花 750ms 只多省 690 字节，不值
pub fn oxipng_optimization(level: u8, lossless: bool) -> u8 {
    if lossless {
        4 // 无损归档：深度
    } else if level <= 1 {
        2 // level 1：轻量无损
    } else if level == 2 {
        4 // level 2：深度无损
    } else {
        1 // 被强制路由到无损引擎的有损档（调色板 PNG）：快速档
    }
}

/// JXL 有损（cjxl）distance：0 = 数学无损，1.0 = 默认视觉无损；v1 固定 1.0（M3 标定）
pub fn jxl_distance(level: u8) -> f64 {
    let _ = level;
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_level_to_jpeg_quality() {
        assert_eq!(jpeg_quality(1), 90);
        assert_eq!(jpeg_quality(4), 60);
        assert_eq!(jpeg_quality(10), 0);
    }

    #[test]
    fn engine_level_to_webp_quality() {
        assert_eq!(webp_quality(5), 75, "web 出厂档 webp q75 依赖此公式");
        assert_eq!(webp_quality(1), 95);
        assert_eq!(webp_quality(10), 50);
    }

    #[test]
    fn engine_level_to_gif_colors() {
        assert_eq!(gif_colors(1), 255);
        assert_eq!(gif_colors(10), 32);
        assert_eq!(gif_colors(4), 181); // round((10-4)/9*223)+32 = round(148.67)+32 = 149+32
    }

    #[test]
    fn engine_level_to_pngquant_params() {
        assert_eq!(pngquant_quality_min(3), 92);
        assert_eq!(pngquant_quality_max(3), 96);
        assert_eq!(pngquant_speed(3), 2); // (3-2)×1.5 = 1.5 → round 2
        assert_eq!(pngquant_quality_min(10), 36);
        assert_eq!(pngquant_quality_max(10), 68);
        assert_eq!(pngquant_speed(10), 11);
    }

    #[test]
    fn engine_oxipng_optimization_map() {
        assert_eq!(oxipng_optimization(1, false), 2);
        assert_eq!(oxipng_optimization(2, false), 4);
        assert_eq!(oxipng_optimization(4, false), 1, "调色板 PNG 等有损档位落到无损引擎时走快速档");
        assert_eq!(oxipng_optimization(4, true), 4, "无损档始终深度优化");
    }
}
