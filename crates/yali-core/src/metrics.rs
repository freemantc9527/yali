//! 可选质量度量：SSIM（对比查看器与 benchkit 同一口径——外部 ffmpeg ssim 滤镜）。
//!
//! 为什么不用 `magick compare -metric SSIM`：本机 ImageMagick 7.1.2-36 的该度量语义
//! 损坏（同像素输入输出 0 而非 1），见 benchkit/README.md。ffmpeg 语义正确（同图=1.0）。
//! HEIC/JXL 等 ffmpeg 解不了的容器：macOS 先 sips 转 PNG 再比。

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;

/// 外部 FFmpeg 的运行时状态。状态不写入 settings.json，因为 PATH 和安装位置会变化。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub available: bool,
    pub path: Option<String>,
    pub ssim: bool,
}

/// 解析可选的外部 ffmpeg：PATH → macOS Homebrew 标准位置。
pub(crate) fn ffmpeg_path() -> Option<PathBuf> {
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        for name in ["ffmpeg", "ffmpeg.exe"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        return ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file());
    }

    #[cfg(not(target_os = "macos"))]
    None
}

async fn probe_ssim_filter(ffmpeg: &Path) -> Option<bool> {
    let mut command = tokio::process::Command::new(ffmpeg);
    command
        .envs(crate::runtime::command_environment(ffmpeg))
        .args(["-hide_banner", "-filters"])
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.split_whitespace().any(|field| field == "ssim")),
    )
}

/// 检测外部 ffmpeg 是否存在且包含 SSIM 滤镜。
pub async fn ffmpeg_status() -> FfmpegStatus {
    let Some(ffmpeg) = ffmpeg_path() else {
        return FfmpegStatus { available: false, path: None, ssim: false };
    };
    let Some(ssim) = probe_ssim_filter(&ffmpeg).await else {
        return FfmpegStatus {
            available: false,
            path: Some(ffmpeg.to_string_lossy().into_owned()),
            ssim: false,
        };
    };
    FfmpegStatus {
        available: true,
        path: Some(ffmpeg.to_string_lossy().into_owned()),
        ssim,
    }
}

pub async fn ssim_available() -> bool {
    ffmpeg_status().await.ssim
}

/// 两图 SSIM（0..=1，1 表示完全一致）；两侧尺寸必须一致
pub async fn ssim(a: &Path, b: &Path) -> Result<f64> {
    let ffmpeg = ffmpeg_path().ok_or_else(|| anyhow!("系统未安装支持 SSIM 的 ffmpeg"))?;
    if let Ok(v) = run_ssim(&ffmpeg, a, b).await {
        return Ok(v);
    }
    // macOS 上 ffmpeg 解不了（HEIC/JXL）：sips 转 PNG 再比。
    #[cfg(not(target_os = "macos"))]
    return Err(anyhow!("当前平台的 ffmpeg 无法直接解码这两张图片"));

    #[cfg(target_os = "macos")]
    {
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (src, dst);
        bail!("当前平台没有 sips 图片解码回退");
    }
    #[cfg(target_os = "macos")]
    {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn ffmpeg_probe_distinguishes_missing_filter_and_failed_process() {
        use std::os::unix::fs::PermissionsExt;

        let workspace = crate::data::TempDir::new().unwrap();
        let tool = workspace.path().join("ffmpeg");
        assert_eq!(probe_ssim_filter(&tool).await, None);
        for (body, expected) in [
            ("printf ' TS ssim VV->V Calculate SSIM\\n'", Some(true)),
            ("printf ' .. ssim360 VV->V Calculate SSIM360\\n'", Some(false)),
            ("exit 1", None),
        ] {
            std::fs::write(&tool, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(probe_ssim_filter(&tool).await, expected);
        }
    }

    fn sample(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../benchkit/samples")
            .join(name)
    }

    #[tokio::test]
    async fn metrics_ssim_identity_is_one() {
        if !ssim_available().await { return; }
        let photo = sample("photo.jpg");
        let v = ssim(&photo, &photo).await.unwrap();
        assert!((v - 1.0).abs() < 1e-6, "同图 SSIM 应为 1，得到 {v}");
    }

    #[tokio::test]
    async fn metrics_ssim_compressed_is_lower() {
        if !ssim_available().await { return; }
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
