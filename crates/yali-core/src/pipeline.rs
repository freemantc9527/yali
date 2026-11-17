//! 压缩管道：读文件 → 魔数嗅探真实格式 → 预缩放(照片类) → 按格式压缩 →
//! 格式转换(如需)再压 → 负优化判定 → 命名落盘 + undo 清单（DESIGN §3）。

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::engines::registry::{run_plan, EncodeRequest, Registry};
use crate::profile::{Profile, QualityKind};
use crate::undo;

/// 魔数嗅探结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
    Tiff,
    Avif,
    Heic,
    Jxl,
}

impl std::fmt::Display for InputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            InputFormat::Jpeg => "jpeg",
            InputFormat::Png => "png",
            InputFormat::Gif => "gif",
            InputFormat::Webp => "webp",
            InputFormat::Svg => "svg",
            InputFormat::Tiff => "tiff",
            InputFormat::Avif => "avif",
            InputFormat::Heic => "heic",
            InputFormat::Jxl => "jxl",
        };
        write!(f, "{s}")
    }
}

pub fn sniff(path: &Path) -> Result<InputFormat> {
    let mut buf = [0u8; 16];
    let mut f = std::fs::File::open(path).with_context(|| format!("无法读取 {}", path.display()))?;
    let n = f.read(&mut buf)?;
    let b = &buf[..n];
    let fmt = if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        InputFormat::Jpeg
    } else if b.starts_with(&[0x89, b'P', b'N', b'G']) {
        InputFormat::Png
    } else if b.starts_with(b"GIF8") {
        InputFormat::Gif
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        InputFormat::Webp
    } else if b.len() >= 12 && &b[4..8] == b"ftyp" {
        match &b[8..12] {
            b"avif" | b"avis" => InputFormat::Avif,
            b"heic" | b"heix" | b"mif1" | b"msf1" | b"hevc" => InputFormat::Heic,
            other => {
                // ftyp 家族里未认识的 brand 当作 HEIC 容器处理（ImageIO 系）
                let brand = String::from_utf8_lossy(other).into_owned();
                bail!("未认识的 ISOBMFF brand '{brand}'，暂不支持");
            }
        }
    } else if b.starts_with(&[0xFF, 0x0A]) || b.starts_with(&[0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ']) {
        InputFormat::Jxl
    } else if b.starts_with(b"II*") || b.starts_with(b"MM") {
        InputFormat::Tiff
    } else {
        // SVG：文本型，允许 BOM/空白前缀（只读头 4KB，不整读）
        let mut head = Vec::new();
        if std::fs::File::open(path)
            .and_then(|f| f.take(4096).read_to_end(&mut head))
            .is_ok()
            && String::from_utf8_lossy(&head)
                .trim_start_matches('\u{feff}')
                .trim_start()
                .to_lowercase()
                .starts_with("<svg")
        {
            InputFormat::Svg
        } else {
            bail!("无法识别的文件格式: {}", path.display());
        }
    };
    Ok(fmt)
}

/// 单文件压缩结果（stdout JSON 摘要，camelCase）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressReport {
    pub input: String,
    pub output: Option<String>,
    pub input_bytes: u64,
    pub output_bytes: Option<u64>,
    pub saved_bytes: Option<i64>,
    pub saved_pct: Option<f64>,
    pub engine: String,
    pub elapsed_ms: u128,
    pub status: &'static str,
}

/// 压缩一个文件（完整管道）
pub async fn compress_file(
    input: &Path,
    profile: &Profile,
    registry: &Registry,
) -> Result<CompressReport> {
    let started = std::time::Instant::now();
    let input_bytes = std::fs::metadata(input)?.len();
    let input_fmt = sniff(input)?;

    // SVG 走 svgo 无质量概念
    let quality = &profile.quality;

    let target_fmt = match profile.format.output {
        crate::profile::OutputFormat::Auto => Registry::auto_of(input_fmt),
        explicit => {
            if explicit == crate::profile::OutputFormat::Svg && input_fmt != InputFormat::Svg {
                bail!("SVG 只支持作为 SVG 输出目标");
            }
            explicit
        }
    };

    // GIF 动图策略：keep → 保持 GIF；webp → gif2webp 显式开启
    let target_fmt = if input_fmt == InputFormat::Gif
        && profile.format.animated == crate::profile::AnimatedPolicy::Webp
        && target_fmt == crate::profile::OutputFormat::Gif
    {
        crate::profile::OutputFormat::Webp
    } else {
        target_fmt
    };

    // 输出落点：目录策略（sibling/downloads/custom/override）+ 后缀模板；
    // {size} 要等编码完成才知道，先按无 size 版本算临时名
    let out_path = final_path(input, profile, target_fmt, None)?;
    let out_dir = out_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("创建输出目录失败: {}", out_dir.display()))?;

    // 容器解码：HEIC/AVIF/JXL → 其他格式时先用 sips（macOS 原生）解成 PNG。
    // 不能走 vips：libvips 的 heif loader 会拿到内嵌缩略图（实测 1920×1280 只解出
    // 128×85），属数据损坏级问题。
    let mut decoded: Option<PathBuf> = None;
    let (mut base_input, mut base_fmt) = (input.to_path_buf(), input_fmt);
    if matches!(input_fmt, InputFormat::Heic | InputFormat::Avif | InputFormat::Jxl)
        && target_fmt != Registry::auto_of(input_fmt)
    {
        let png = decode_to_png_via_sips(input, &out_dir, registry).await?;
        base_input = png.clone();
        base_fmt = InputFormat::Png;
        decoded = Some(png);
    }

    let encoder = registry.pick(base_fmt, target_fmt)?;

    // 预缩放（照片类）：maxWidth/maxHeight 且 mode != none → vipsthumbnail 中转
    let encode_input: PathBuf = match pre_scaled_copy(&base_input, base_fmt, profile, registry).await? {
        Some(tmp) => tmp,
        None => base_input.clone(),
    };

    // 编码：level 档单次；targetBytes 走 ≤5 轮 level 二分（收敛 [target-10KB, target]，
    // 受 minQualityFloor 保护）；lossless 单次
    let tmp = temp_sibling(&out_path);
    let encode_once = |level: Option<u8>, lossless: bool| {
        let plan = encoder.plan(&EncodeRequest {
            input: encode_input.clone(),
            output: tmp.clone(),
            input_fmt: base_fmt,
            output_fmt: target_fmt,
            level,
            lossless,
        });
        let cleanup = tmp.clone();
        async move {
            let plan = plan?;
            if let Err(e) = run_plan(&plan).await {
                let _ = std::fs::remove_file(&cleanup); // 失败不残留临时文件
                return Err(e);
            }
            Ok::<(), anyhow::Error>(())
        }
    };

    let final_level: Option<u8> = match quality.kind {
        QualityKind::Level => Some(quality.level.unwrap_or(4)),
        QualityKind::Lossless => None,
        QualityKind::TargetBytes => {
            let target = quality.target_bytes.unwrap_or(500_000);
            let mut lo = quality.min_quality_floor.clamp(1, 10);
            let mut hi = 10u8;
            let mut best: Option<u8> = None;
            let mut encoded: Option<u8> = None; // tmp 当前内容对应的 level
            for _ in 0..5 {
                if lo > hi {
                    break;
                }
                let mid = (lo + hi) / 2;
                encode_once(Some(mid), false).await?;
                encoded = Some(mid);
                let size = std::fs::metadata(&tmp)?.len();
                if size > target {
                    lo = mid + 1; // 偏大 → 提高强度
                } else {
                    best = Some(mid);
                    if size < target.saturating_sub(10_000) {
                        hi = mid.saturating_sub(1); // 偏小 → 降低强度
                    } else {
                        break; // 收敛区间 [target-10KB, target]
                    }
                }
            }
            match best {
                Some(chosen) => {
                    // tmp 内容可能来自最后一轮而非 best：不一致则重编一次
                    if encoded != Some(chosen) {
                        encode_once(Some(chosen), false).await?;
                    }
                    Some(chosen)
                }
                None => {
                    // 一直偏大：压到最强档（受 floor 保护过，仍偏大则如实输出）
                    encode_once(Some(10), false).await?;
                    Some(10)
                }
            }
        }
    };

    // tmp 里已是目标结果就不再重编（bisection 收敛场景），否则编码一次
    if quality.kind != QualityKind::TargetBytes {
        encode_once(final_level, quality.kind == QualityKind::Lossless).await?;
    }
    let output_size = std::fs::metadata(&tmp)?.len();

    // 尺寸守卫：编码器悄悄降采样（如 vips 解码容器只拿到缩略图）必须当场拦下
    if let (Some((iw, ih)), Some((ow, oh))) = (probe_dims(&encode_input).await, probe_dims(&tmp).await) {
        if (iw, ih) != (ow, oh) {
            let _ = std::fs::remove_file(&tmp);
            bail!(
                "输出尺寸异常：{}×{} → {}×{}（编码器疑似降采样，已拒绝落盘）",
                iw, ih, ow, oh
            );
        }
    }

    // 负优化拦截：同格式且输出 ≥ 输入 → 不落盘（DESIGN §3）
    let same_format = target_fmt == Registry::auto_of(input_fmt);
    let mut status = "done";
    if same_format && output_size >= input_bytes {
        let _ = std::fs::remove_file(&tmp);
        status = "larger_output";
    }

    // 清理中转（预缩放 PNG / 容器解码 PNG）
    if encode_input != input {
        let _ = std::fs::remove_file(&encode_input);
    }
    if let Some(png) = decoded {
        if png != encode_input {
            let _ = std::fs::remove_file(&png);
        }
    }

    let mut report_output: Option<PathBuf> = None;
    let mut report_bytes: Option<u64> = Some(output_size);
    if status == "done" {
        // {size} 占位在编码完成后渲染
        let final_path = final_path(input, profile, target_fmt, Some(output_size))?;
        std::fs::rename(&tmp, &final_path)?;
        if profile.output.undo_manifest {
            undo::append_entry(
                &final_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
                input,
                &final_path,
            )?;
        }
        report_output = Some(final_path);
    } else {
        report_bytes = None;
    }
    let saved_bytes = report_bytes.map(|o| input_bytes as i64 - o as i64);
    let saved_pct = saved_bytes.map(|s| (s as f64 / input_bytes.max(1) as f64) * 100.0);

    Ok(CompressReport {
        input: input.to_string_lossy().into(),
        output: report_output.map(|p| p.to_string_lossy().into()),
        input_bytes,
        output_bytes: report_bytes,
        saved_bytes,
        saved_pct,
        engine: encoder.name().into(),
        elapsed_ms: started.elapsed().as_millis(),
        status,
    })
}

/// 预缩放：vipsthumbnail（从不放大，天然 shrinkOnly）→ 无损 PNG 中转
async fn pre_scaled_copy(
    input: &Path,
    input_fmt: InputFormat,
    profile: &Profile,
    registry: &Registry,
) -> Result<Option<PathBuf>> {
    if profile.resize.mode == crate::profile::ResizeMode::None {
        return Ok(None);
    }
    if profile.resize.max_width.is_none() && profile.resize.max_height.is_none() {
        return Ok(None);
    }
    if matches!(input_fmt, InputFormat::Svg | InputFormat::Gif) {
        return Ok(None); // 矢量与动图不做预缩放
    }
    let vips = registry
        .path_of(crate::engines::EngineKind::Vips)
        .ok_or_else(|| anyhow::anyhow!("预缩放需要 vips（brew install vips）"))?;
    // 注意：vipsthumbnail 不认 dotfile 输出路径（会把图片写到 stdout），
    // 中转文件必须是非隐藏名
    let dir = input.parent().unwrap_or(Path::new("."));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("img");
    let tmp = dir.join(format!("yali-{}-{stem}-resize.png", std::process::id()));
    let mut size_spec = String::new();
    if let Some(w) = profile.resize.max_width {
        size_spec.push_str(&format!("{w}"));
    }
    size_spec.push('x');
    if let Some(h) = profile.resize.max_height {
        size_spec.push_str(&h.to_string());
    }
    let plan = crate::engines::registry::SubprocessPlan {
        program: vips,
        args: vec![
            input.to_string_lossy().into(),
            "--size".into(),
            size_spec,
            "-o".into(),
            tmp.to_string_lossy().into(),
        ],
    };
    run_plan(&plan).await?;
    if !tmp.exists() {
        bail!("vips 预缩放未产出文件（vipsthumbnail 行为异常）");
    }
    Ok(Some(tmp))
}

/// 容器解码：sips 把 HEIC/AVIF/JXL 解成 PNG（macOS 原生全分辨率解码）。
/// 不用 vips：libvips heif loader 会返回内嵌缩略图（128×85 事故，已入库坑账本）。
async fn decode_to_png_via_sips(
    input: &Path,
    dir: &Path,
    registry: &Registry,
) -> Result<PathBuf> {
    let sips = registry
        .path_of(crate::engines::EngineKind::Sips)
        .unwrap_or_else(|| PathBuf::from("/usr/bin/sips"));
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("img");
    let png = dir.join(format!("{stem}.yali-{}.decode.png", std::process::id()));
    let plan = crate::engines::registry::SubprocessPlan {
        program: sips,
        args: vec![
            "-s".into(),
            "format".into(),
            "png".into(),
            input.to_string_lossy().into(),
            "--out".into(),
            png.to_string_lossy().into(),
        ],
    };
    run_plan(&plan).await?;
    if !png.exists() {
        bail!("sips 容器解码失败: {}", input.display());
    }
    Ok(png)
}

/// 像素尺寸探测（sips -g；失败返回 None，尺寸守卫降级为跳过）
async fn probe_dims(path: &Path) -> Option<(u32, u32)> {
    let out = tokio::process::Command::new("sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(path)
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut w = None;
    let mut h = None;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("pixelWidth:") {
            w = v.trim().parse().ok();
        }
        if let Some(v) = line.trim().strip_prefix("pixelHeight:") {
            h = v.trim().parse().ok();
        }
    }
    Some((w?, h?))
}

/// 输出目录策略（DESIGN §2 output.dir / T-008）
fn output_dir(input: &Path, profile: &Profile) -> Result<PathBuf> {
    use crate::profile::DirKind;
    match profile.output.dir {
        DirKind::Sibling => Ok(input.parent().unwrap_or(Path::new(".")).to_path_buf()),
        // 单次覆盖（workflow/CLI 场景）：环境变量优先，未设置时回落 sibling
        DirKind::Override => match std::env::var("YALI_OUTPUT_DIR") {
            Ok(dir) if !dir.trim().is_empty() => expand_home(&dir),
            _ => Ok(input.parent().unwrap_or(Path::new(".")).to_path_buf()),
        },
        DirKind::Custom => {
            let raw = profile
                .output
                .dir_path
                .as_deref()
                .context("output.dir=custom 但缺少 dirPath（validate 应已拦截）")?;
            expand_home(raw)
        }
        DirKind::Downloads => {
            let home = std::env::var("HOME").context("HOME 未设置，无法定位下载目录")?;
            Ok(PathBuf::from(home).join("Downloads"))
        }
    }
}

fn expand_home(raw: &str) -> Result<PathBuf> {
    if let Some(rest) = raw.strip_prefix("~/") {
        let home = std::env::var("HOME").context("HOME 未设置，无法展开 ~")?;
        return Ok(PathBuf::from(home).join(rest));
    }
    if raw == "~" {
        let home = std::env::var("HOME").context("HOME 未设置，无法展开 ~")?;
        return Ok(PathBuf::from(home));
    }
    Ok(PathBuf::from(raw))
}

/// `{size}` 的人类可读值（编码完成后才可用）
fn size_token(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{}K", bytes / 1024)
    } else {
        format!("{bytes}B")
    }
}

fn ext_of(target_fmt: crate::profile::OutputFormat) -> Result<&'static str> {
    Ok(match target_fmt {
        crate::profile::OutputFormat::Jpeg => "jpg",
        crate::profile::OutputFormat::Png => "png",
        crate::profile::OutputFormat::Gif => "gif",
        crate::profile::OutputFormat::Webp => "webp",
        crate::profile::OutputFormat::Avif => "avif",
        crate::profile::OutputFormat::Heic => "heic",
        crate::profile::OutputFormat::Jxl => "jxl",
        crate::profile::OutputFormat::Svg => "svg",
        crate::profile::OutputFormat::Auto => bail!("auto 不应出现在最终命名"),
    })
}

/// 最终输出路径：目录策略 + 后缀模板（{profile}/{date}/{size}）。
/// `size` 为 None 时保留 `{size}` 占位（用于编码前的临时命名）。
fn final_path(
    input: &Path,
    profile: &Profile,
    target_fmt: crate::profile::OutputFormat,
    size: Option<u64>,
) -> Result<PathBuf> {
    let dir = output_dir(input, profile)?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .context("文件名不是合法 UTF-8")?;
    let ext = ext_of(target_fmt)?;
    let mut suffix = profile
        .output
        .suffix_template
        .replace("{profile}", &profile.name)
        .replace("{date}", &date_stamp());
    if let Some(bytes) = size {
        suffix = suffix.replace("{size}", &size_token(bytes));
    }
    let name = format!("{stem}{suffix}.{ext}");
    if Path::new(&name) == input.file_name().unwrap_or_default() && dir == input.parent().unwrap_or(Path::new(".")) {
        bail!("后缀模板渲染后与输入同名，会覆盖原文件");
    }
    Ok(dir.join(name))
}

fn date_stamp() -> String {
    // 无 chrono 依赖：用 UNIX 时间秒换算近似日戳（M1 引 chrono 或 time crate 再精确化）
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86_400;
    // 2026-01-01 是 UNIX 第 20454 天
    let day_index = days as i64 - 20_454;
    let (y, m, d) = civil_from_days(day_index);
    format!("{y:04}{m:02}{d:02}")
}

/// days since 2026-01-01 → (y, m, d)（公历换算，Howard Hinnant 算法简化版）
fn civil_from_days(day_index: i64) -> (i64, u32, u32) {
    let mut days = day_index;
    let mut year = 2026i64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let len = if leap { 366 } else { 365 };
        if days < len {
            break;
        }
        days -= len;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_lens = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    while days >= month_lens[month] {
        days -= month_lens[month];
        month += 1;
    }
    (year, (month + 1) as u32, (days + 1) as u32)
}

/// 同目录临时文件名（保 rename 原子性）。
/// 注意两点引擎约束：①vipsthumbnail 不认 dotfile 输出路径，故不用隐藏名；
/// ②vipsthumbnail 按扩展名推断输出格式，故保留目标扩展名在末尾。
fn temp_sibling(target: &Path) -> PathBuf {
    let pid = std::process::id();
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let ext = target.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    target.with_file_name(format!("{stem}.yali-{pid}.tmp.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::DirKind;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn pipeline_sniff_common_formats() {
        let dir = std::env::temp_dir().join(format!("yali-sniff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cases: Vec<(&[u8], InputFormat)> = vec![
            (&[0xFF, 0xD8, 0xFF, 0xE0], InputFormat::Jpeg),
            (&[0x89, b'P', b'N', b'G', 0x0D, 0x0A], InputFormat::Png),
            (b"GIF89a", InputFormat::Gif),
        ];
        for (magic, expect) in cases {
            let p = dir.join(format!("s{:?}.bin", expect));
            std::fs::write(&p, magic).unwrap();
            assert_eq!(sniff(&p).unwrap(), expect);
        }
        // webp: RIFF....WEBP
        let mut webp = b"RIFF\x00\x00\x00\x00".to_vec();
        webp.extend_from_slice(b"WEBPVP8 ");
        let p = dir.join("s.webp");
        std::fs::write(&p, webp).unwrap();
        assert_eq!(sniff(&p).unwrap(), InputFormat::Webp);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pipeline_sniff_ftyp_brands() {
        let dir = std::env::temp_dir().join(format!("yali-sniff2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut avif = vec![0u8; 12];
        avif[4..8].copy_from_slice(b"ftyp");
        avif[8..12].copy_from_slice(b"avif");
        let p = dir.join("s.avif");
        std::fs::write(&p, avif).unwrap();
        assert_eq!(sniff(&p).unwrap(), InputFormat::Avif);

        let mut heic = vec![0u8; 12];
        heic[4..8].copy_from_slice(b"ftyp");
        heic[8..12].copy_from_slice(b"heic");
        let p = dir.join("s.heic");
        std::fs::write(&p, heic).unwrap();
        assert_eq!(sniff(&p).unwrap(), InputFormat::Heic);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn pipeline_negative_optimization_decision() {
        // 纯规则：同格式 && 输出 ≥ 输入 → 拦截
        let intercept = |same: bool, out: u64, input: u64| same && out >= input;
        assert!(intercept(true, 100, 100));
        assert!(intercept(true, 150, 100));
        assert!(!intercept(false, 150, 100), "跨格式转换不做体积比较");
        assert!(!intercept(true, 90, 100));
    }

    #[test]
    fn pipeline_suffix_template_placeholders() {
        let mut profile = Profile::default();
        profile.name = "web".into();
        let input = Path::new("/tmp/photo.jpg");
        let out = final_path(input, &profile, crate::profile::OutputFormat::Webp, None).unwrap();
        assert_eq!(out.file_name().unwrap(), "photo-web.webp");
    }

    #[test]
    fn pipeline_output_dir_strategies() {
        let _guard = ENV_LOCK.lock().unwrap();
        let original_home = std::env::var("HOME").ok();
        let original_override = std::env::var("YALI_OUTPUT_DIR").ok();
        std::env::set_var("HOME", "/home/tester");
        std::env::remove_var("YALI_OUTPUT_DIR");

        let input = Path::new("/tmp/photo/photo.jpg");
        let mut profile = Profile::default();
        profile.name = "t".into();

        // sibling：与输入同目录
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/photo"));

        // downloads：$HOME/Downloads
        profile.output.dir = DirKind::Downloads;
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/home/tester/Downloads"));

        // custom：绝对路径 / ~ 展开
        profile.output.dir = DirKind::Custom;
        profile.output.dir_path = Some("/Users/x/out".into());
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/Users/x/out"));
        profile.output.dir_path = Some("~/压缩输出".into());
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/home/tester/压缩输出"));

        // override：环境变量优先，缺失回落 sibling
        profile.output.dir = DirKind::Override;
        std::env::set_var("YALI_OUTPUT_DIR", "/tmp/override-dir");
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/override-dir"));
        std::env::remove_var("YALI_OUTPUT_DIR");
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/photo"));

        match original_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        if let Some(v) = original_override {
            std::env::set_var("YALI_OUTPUT_DIR", v);
        }
    }

    #[test]
    fn pipeline_suffix_size_placeholder_rendered_after_encode() {
        let mut profile = Profile::default();
        profile.name = "web".into();
        profile.output.suffix_template = "-{profile}-{size}".into();
        let input = Path::new("/tmp/photo.jpg");
        // 编码前：{size} 原样保留（仅用于临时命名）
        let pre = final_path(input, &profile, crate::profile::OutputFormat::Jpeg, None).unwrap();
        assert_eq!(pre.file_name().unwrap(), "photo-web-{size}.jpg");
        // 编码后：按实际体积渲染
        let post = final_path(input, &profile, crate::profile::OutputFormat::Jpeg, Some(49_178)).unwrap();
        assert_eq!(post.file_name().unwrap(), "photo-web-48K.jpg");
        let mb = final_path(input, &profile, crate::profile::OutputFormat::Jpeg, Some(2_500_000)).unwrap();
        assert_eq!(mb.file_name().unwrap(), "photo-web-2.4M.jpg");
    }

    #[test]
    fn pipeline_same_name_template_rejected() {
        let mut profile = Profile::default();
        profile.output.suffix_template = String::new();
        let input = Path::new("/tmp/photo.jpg");
        assert!(final_path(input, &profile, crate::profile::OutputFormat::Jpeg, None).is_err());
    }
}
