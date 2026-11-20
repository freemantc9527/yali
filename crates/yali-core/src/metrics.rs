//! 质量度量：SSIM（对比查看器与 benchkit 同一口径——ffmpeg ssim 滤镜）。
//!
//! 为什么不用 `magick compare -metric SSIM`：本机 ImageMagick 7.1.2-36 的该度量语义
//! 损坏（同像素输入输出 0 而非 1），见 benchkit/README.md。ffmpeg 语义正确（同图=1.0）。
//! HEIC/JXL 等 ffmpeg 解不了的容器：先 sips 转 PNG 再比。

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};

/// 解析 ffmpeg：PATH → Homebrew 标准位置（Services 上下文 PATH 极简）
pub(crate) fn ffmpeg_path() -> Option<PathBuf> {
    if crate::runtime::contents().is_some() {
        return crate::runtime::bundled_binary("ffmpeg");
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|s| !s.is_empty()) {
            let candidate = Path::new(dir).join("ffmpeg");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

/// 两图 SSIM（0..=1，1 表示完全一致）；两侧尺寸必须一致
pub async fn ssim(a: &Path, b: &Path) -> Result<f64> {
    let ffmpeg = ffmpeg_path().ok_or_else(|| anyhow!("ffmpeg 不可用，请重新构建或安装完整鸭梨应用"))?;
    if let Ok(v) = run_ssim(&ffmpeg, a, b).await {
        return Ok(v);
    }
    // ffmpeg 解不了（HEIC/JXL）：sips 转 PNG 再比
    let workspace = crate::data::TempDir::new()?;
    let pa = workspace.path().join("ssim-a.png");
    let pb = workspace.path().join("ssim-b.png");
    to_png(a, &pa).await?;
    to_png(b, &pb).await?;
    let result = run_ssim(&ffmpeg, &pa, &pb).await;
    let _ = std::fs::remove_file(&pa);
    let _ = std::fs::remove_file(&pb);
    result
}

async fn run_ssim(ffmpeg: &Path, a: &Path, b: &Path) -> Result<f64> {
    let out = tokio::process::Command::new(ffmpeg)
        .envs(crate::runtime::command_environment(ffmpeg))
        .args(["-hide_banner", "-loglevel", "info", "-i"])
        .arg(a)
        .arg("-i")
        .arg(b)
        .args(["-lavfi", "ssim", "-f", "null", "-"])
        .output()
        .await
        .context("执行 ffmpeg 失败")?;
    let text = String::from_utf8_lossy(&out.stderr);
    let value = text
        .split("All:")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| anyhow!("ffmpeg 未输出 SSIM（两侧尺寸不一致或格式不支持）"))?;
    if !(0.0..=1.0).contains(&value) {
        bail!("SSIM 越界: {value}");
    }
    Ok(value)
}

async fn to_png(src: &Path, dst: &Path) -> Result<()> {
    let sips = ["/usr/bin/sips"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
        .ok_or_else(|| anyhow!("sips 不可用"))?;
    let out = tokio::process::Command::new(sips)
        .args(["-s", "format", "png"])
        .arg(src)
        .arg("--out")
        .arg(dst)
        .output()
        .await?;
    if !out.status.success() || !dst.exists() {
        bail!("sips 转码失败: {}", src.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchkit/samples")
            .join(name)
    }

    #[tokio::test]
    async fn metrics_ssim_identity_is_one() {
        let photo = sample("photo.jpg");
        let v = ssim(&photo, &photo).await.unwrap();
        assert!((v - 1.0).abs() < 1e-6, "同图 SSIM 应为 1，得到 {v}");
    }

    #[tokio::test]
    async fn metrics_ssim_compressed_is_lower() {
        // 用已入库的 pngquant 产物？没有——就地生成一张低质量副本再比
        let photo = sample("photo.jpg");
        let dir = std::env::temp_dir().join(format!("yali-ssim-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let low = dir.join("low.jpg");
        let cjpeg = Path::new("/opt/homebrew/opt/mozjpeg/bin/cjpeg");
        if cjpeg.is_file() {
            let status = std::process::Command::new(cjpeg)
                .args(["-quality", "20", "-outfile"])
                .arg(&low)
                .arg(&photo)
                .status()
                .unwrap();
            assert!(status.success());
            let v = ssim(&photo, &low).await.unwrap();
            assert!(v < 0.95 && v > 0.0, "低质量副本 SSIM 应明显小于 1，得到 {v}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
