//! 壳层认识的图片扩展名（open-file / watch 共用一份，别再各写一遍）。

use std::path::Path;

/// 与 `tauri.conf.json` 的 fileAssociations 对齐
#[cfg(not(windows))]
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "heic", "heif", "gif", "webp", "avif", "jxl", "tif", "tiff", "bmp",
];

#[cfg(windows)]
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp"];

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}
