//! 压缩管道：读文件 → 魔数嗅探真实格式 → 预缩放(照片类) → 按格式压缩 →
//! 格式转换(如需)再压 → 负优化判定 → 命名落盘 + undo 清单（DESIGN §3）。

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::engines::registry::{run_plan, EncodeRequest, Registry, SubprocessPlan, EngineExit};
use crate::engines::EngineKind;
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
    Bmp,
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
            InputFormat::Bmp => "bmp",
            InputFormat::Avif => "avif",
            InputFormat::Heic => "heic",
            InputFormat::Jxl => "jxl",
        };
        write!(f, "{s}")
    }
}

fn input_supported(format: InputFormat, windows: bool) -> bool {
    !windows || !matches!(format, InputFormat::Svg | InputFormat::Avif | InputFormat::Heic | InputFormat::Jxl)
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
    } else if b.starts_with(b"BM") {
        InputFormat::Bmp
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
    if !input_supported(fmt, cfg!(windows)) {
        bail!("Windows 不支持 {fmt} 图片");
    }
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
    #[cfg(windows)]
    if matches!(profile.format.output, crate::profile::OutputFormat::Avif | crate::profile::OutputFormat::Heic | crate::profile::OutputFormat::Jxl | crate::profile::OutputFormat::Svg) {
        bail!("Windows 不支持此输出格式");
    }
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
    if !out_dir.is_dir() {
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("创建输出目录失败: {}", out_dir.display()))?;
    }
    let workspace = crate::data::TempDir::new()?;

    // 容器解码：HEIC/AVIF/JXL → 其他格式时先用 sips（macOS 原生）解成 PNG。
    // 不能走 vips：libvips 的 heif loader 会拿到内嵌缩略图（实测 1920×1280 只解出
    // 128×85），属数据损坏级问题。
    let mut decoded: Option<PathBuf> = None;
    let (mut base_input, mut base_fmt) = (input.to_path_buf(), input_fmt);
    if input_fmt == InputFormat::Bmp
        || (matches!(input_fmt, InputFormat::Heic | InputFormat::Avif | InputFormat::Jxl)
            && target_fmt != Registry::auto_of(input_fmt))
    {
        let png = decode_to_png_via_sips(input, workspace.path(), registry).await?;
        base_input = png.clone();
        base_fmt = InputFormat::Png;
        decoded = Some(png);
    }

    // PNG → JPEG：先把 alpha 无损压平到白底再交给 mozjpeg。
    // 原因：cjpeg 与 libvips 直吃 RGBA 都合成到**黑底**（本机实测 PSNR 2.2dB），
    // 而 sips 直出虽正确但体积比 mozjpeg 大 2.6 倍；分享截图既要白底又要小。
    // libvips flatten 的 background 用 16 位标度，65535 才是白。
    // PNG → JPEG：只有真的带透明才需要压平 alpha（vips flatten 对无 alpha 的
    // 调色板 PNG 会输出 graya 灰白图——实测毁掉整张截图，所以必须条件化）。
    // 压平失败（vips 缺失/报错）时回落 sips 直出，绝不回落「cjpeg 直吃 RGBA」（黑底）。
    let mut aux_file: Option<PathBuf> = None;
    let mut forced_encoder: Option<crate::engines::EngineKind> = None;
    if base_fmt == InputFormat::Png && target_fmt == crate::profile::OutputFormat::Jpeg {
        if png_has_alpha(&base_input) {
            match flatten_alpha_to_white(&base_input, workspace.path()).await {
                Ok(Some(flat)) => {
                    aux_file = Some(flat);
                    base_input = aux_file.clone().unwrap();
                }
                _ => forced_encoder = Some(crate::engines::EngineKind::Sips),
            }
        }
    }

    // 调色板 PNG → PNG：强制无损引擎（oxipng），不做二次量化
    if base_fmt == InputFormat::Png && target_fmt == crate::profile::OutputFormat::Png {
        // BMP 默认转无损 PNG，避免对照片先量化而达不到 pngquant 最低质量。
        if input_fmt == InputFormat::Bmp
            || (quality.kind == QualityKind::Recommended && png_is_animated(&base_input))
            || png_png_engine_choice(png_color_type(&base_input)) == crate::engines::EngineKind::Oxipng {
            forced_encoder = Some(crate::engines::EngineKind::Oxipng);
        }
    }

    let encoder = match forced_encoder.and_then(|k| registry.encoder_of(k)) {
        Some(e) => e,
        None => registry.pick(base_fmt, target_fmt)?,
    };

    // 预缩放（照片类）：maxWidth/maxHeight 且 mode != none → vipsthumbnail 中转
    let encode_input: PathBuf =
        match pre_scaled_copy(&base_input, base_fmt, profile, registry, workspace.path()).await? {
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
            recommended: quality.kind == QualityKind::Recommended,
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
        QualityKind::Recommended => Some(4), // 其他格式沿用旧默认档；JPEG / PNG 有固定推荐参数
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
    let mut engine_name = encoder.name().to_string();
    if quality.kind != QualityKind::TargetBytes {
        if let Err(error) = encode_once(final_level, quality.kind == QualityKind::Lossless).await {
            // 只对 pngquant 明确的「达不到质量下限」回退；损坏文件与其他错误照常报错。
            if quality.kind == QualityKind::Recommended
                && encoder.kind() == EngineKind::Pngquant
                && error.downcast_ref::<EngineExit>().is_some_and(|exit| exit.0 == Some(99))
            {
                let fallback = registry.encoder_of(EngineKind::Oxipng)
                    .context("PNG 量化未达质量下限，且 oxipng 不可用")?;
                let plan = fallback.plan(&EncodeRequest {
                    input: encode_input.clone(), output: tmp.clone(),
                    input_fmt: base_fmt, output_fmt: target_fmt,
                    level: Some(4), lossless: false, recommended: true,
                })?;
                if let Err(error) = run_plan(&plan).await {
                    let _ = std::fs::remove_file(&tmp);
                    return Err(error);
                }
                engine_name = fallback.name().into();
            } else {
                return Err(error);
            }
        }
    }
    if quality.kind == QualityKind::Recommended
        && matches!(target_fmt, crate::profile::OutputFormat::Jpeg | crate::profile::OutputFormat::Png)
    {
        if let Err(error) = validate_candidate(&encode_input, &tmp, target_fmt).await {
            let _ = std::fs::remove_file(&tmp);
            return Err(error);
        }
        let candidate = temp_sibling(&tmp); // 与产物同分区，候选发布可原子替换
        let plan = if target_fmt == crate::profile::OutputFormat::Png
            && encoder.kind() == EngineKind::Pngquant && engine_name == "pngquant"
        {
            registry.encoder_of(EngineKind::Oxipng).map(|optimizer| optimizer.plan(&EncodeRequest {
                input: tmp.clone(), output: candidate.clone(), input_fmt: InputFormat::Png,
                output_fmt: target_fmt, level: Some(4), lossless: false, recommended: true,
            })).transpose()?
        } else if target_fmt == crate::profile::OutputFormat::Jpeg
            && sniff(&encode_input)? == InputFormat::Jpeg
        {
            // 必须使用实际编码输入；缩放 / 转换后不能拿原始 JPEG 来替换结果。
            EngineKind::Mozjpeg.sibling_binary("jpegtran").map(|program| SubprocessPlan {
                program,
                args: vec!["-copy".into(), "all".into(), "-optimize".into(), "-progressive".into(),
                    "-outfile".into(), candidate.to_string_lossy().into(), encode_input.to_string_lossy().into()],
            })
        } else { None };
        if let Some(plan) = plan {
            // 可选优化失败不损坏已验证的主产物。
            if run_plan(&plan).await.is_ok()
                && choose_smaller_candidate(&encode_input, &tmp, &candidate, target_fmt).await.unwrap_or(false)
            {
                engine_name = if target_fmt == crate::profile::OutputFormat::Png {
                    "pngquant+oxipng".into()
                } else { "jpegtran".into() };
            }
        }
        let _ = std::fs::remove_file(&candidate);
    }
    let output_size = std::fs::metadata(&tmp)?.len();

    // 尺寸守卫：编码器悄悄降采样（如 vips 解码容器只拿到缩略图）必须当场拦下。
    // 只在水土不服的路径上跑（容器格式 / vips 兜底 / 预缩放），否则每次要多两个 sips
    // 进程，小图耗时实测劣化 4~5%——专用编码器路径不需要这层保险。
    let guarded = encoder.kind() == crate::engines::EngineKind::Vips
        || matches!(input_fmt, InputFormat::Heic | InputFormat::Avif | InputFormat::Jxl)
        || matches!(
            target_fmt,
            crate::profile::OutputFormat::Heic
                | crate::profile::OutputFormat::Avif
                | crate::profile::OutputFormat::Jxl
        )
        || encode_input != input;
    if guarded {
        let (before, after) = tokio::join!(probe_dims(&encode_input), probe_dims(&tmp));
        if let (Some((iw, ih)), Some((ow, oh))) = (before, after) {
            if (iw, ih) != (ow, oh) {
                let _ = std::fs::remove_file(&tmp);
                bail!(
                    "输出尺寸异常：{}×{} → {}×{}（编码器疑似降采样，已拒绝落盘）",
                    iw,
                    ih,
                    ow,
                    oh
                );
            }
        }
    }

    // 负优化拦截：同格式且输出 ≥ 输入 → 不落盘（DESIGN §3）
    let same_format = target_fmt == Registry::auto_of(input_fmt);
    let mut status = "done";
    if same_format && output_size >= input_bytes {
        let _ = std::fs::remove_file(&tmp);
        status = "larger_output";
    }

    // 清理中转（压平 PNG / 预缩放 PNG / 容器解码 PNG）
    if let Some(flat) = aux_file {
        let _ = std::fs::remove_file(&flat);
    }
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
        engine: engine_name,
        elapsed_ms: started.elapsed().as_millis(),
        status,
    })
}

/// 推荐模式的候选必须真实可解码、格式正确，且不改变编码输入的尺寸。
async fn validate_candidate(input: &Path, output: &Path, target: crate::profile::OutputFormat) -> Result<()> {
    if Registry::auto_of(sniff(output)?) != target {
        bail!("候选输出格式异常");
    }
    let (before, after) = tokio::join!(probe_dims(input), probe_dims(output));
    if before.is_none() || before != after {
        bail!("候选输出尺寸异常或无法读取");
    }
    // FFmpeg 是可选的额外解码校验；没有它时，格式嗅探和尺寸守卫仍然生效。
    if let Some(ffmpeg) = crate::metrics::ffmpeg_path() {
        run_plan(&SubprocessPlan { program: ffmpeg, args: vec![
            "-v".into(), "error".into(), "-xerror".into(), "-i".into(), output.to_string_lossy().into(),
            "-f".into(), "null".into(), "-".into(),
        ] }).await?;
    }
    Ok(())
}

async fn choose_smaller_candidate(input: &Path, output: &Path, candidate: &Path, target: crate::profile::OutputFormat) -> Result<bool> {
    if std::fs::metadata(candidate).map(|m| m.len()).unwrap_or(u64::MAX)
        >= std::fs::metadata(output)?.len()
        || validate_candidate(input, candidate, target).await.is_err()
    {
        return Ok(false);
    }
    std::fs::rename(candidate, output)?;
    Ok(true)
}

fn png_is_animated(path: &Path) -> bool {
    let Some(head) = png_head(path) else { return false };
    let mut pos = 8;
    while pos + 12 <= head.len() {
        let size = u32::from_be_bytes(head[pos..pos + 4].try_into().unwrap()) as usize;
        if &head[pos + 4..pos + 8] == b"acTL" { return true; }
        if &head[pos + 4..pos + 8] == b"IDAT" { break; }
        pos = match pos.checked_add(12).and_then(|v| v.checked_add(size)) {
            Some(next) => next, None => break,
        };
    }
    false
}

/// 预缩放：vipsthumbnail → 无损 PNG 中转，shrinkOnly 必须显式禁止放大。
async fn pre_scaled_copy(
    input: &Path,
    input_fmt: InputFormat,
    profile: &Profile,
    registry: &Registry,
    dir: &Path,
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
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("img");
    let tmp = dir.join(format!("yali-{}-{stem}-resize.png", std::process::id()));
    let mut size_spec = String::new();
    if let Some(w) = profile.resize.max_width {
        size_spec.push_str(&format!("{w}"));
    }
    size_spec.push('x');
    if let Some(h) = profile.resize.max_height {
        size_spec.push_str(&h.to_string());
    }
    if profile.resize.mode == crate::profile::ResizeMode::ShrinkOnly {
        size_spec.push('>');
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

/// PNG 头部（tRNS/PLTE/IHDR 都在 IDAT 之前，读前 64KB 足够）
fn png_head(path: &Path) -> Option<Vec<u8>> {
    let mut head = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(64 * 1024)
        .read_to_end(&mut head)
        .ok()?;
    if head.len() < 26 || &head[0..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    Some(head)
}

/// PNG→PNG 用哪个引擎：已经是调色板（type 3）就别二次量化——pngquant 会因
/// 「达不到最低质量」直接退出 99（实测），而且二次量化纯掉质量。改走 oxipng 无损。
fn png_png_engine_choice(color_type: Option<u8>) -> crate::engines::EngineKind {
    match color_type {
        Some(3) => crate::engines::EngineKind::Oxipng,
        _ => crate::engines::EngineKind::Mozjpeg, // 占位：非调色板仍由 registry 按 level 选
    }
}

/// PNG IHDR 的 color type：0=灰度 2=RGB 3=调色板 4=灰度+alpha 6=RGBA
fn png_color_type(path: &Path) -> Option<u8> {
    png_head(path).map(|head| head[25])
}

/// PNG 是否真的带透明：color type 4/6 直接算；type 3（调色板）要看有没有 tRNS
/// chunk（pngquant 无 alpha 的输出就是纯 type 3，对它做 flatten 会毁图）
fn png_has_alpha(path: &Path) -> bool {
    let Some(head) = png_head(path) else { return false };
    let color_type = head[25];
    if matches!(color_type, 4 | 6) {
        return true;
    }
    if color_type != 3 {
        return false;
    }
    let mut off = 8;
    while off + 12 <= head.len() {
        let len =
            u32::from_be_bytes([head[off], head[off + 1], head[off + 2], head[off + 3]]) as usize;
        match &head[off + 4..off + 8] {
            b"tRNS" => return true,
            b"IDAT" => return false, // tRNS 一定排在 IDAT 之前
            _ => {}
        }
        off += 12 + len;
    }
    false
}

/// 用 vips 把 alpha 压平到白底（无损）；vips 不可用时返回 None，由调用方回落
async fn flatten_alpha_to_white(input: &Path, dir: &Path) -> Result<Option<PathBuf>> {
    let Some(vips_cli) = crate::engines::EngineKind::Vips.sibling_binary("vips") else {
        return Ok(None);
    };
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("img");
    let flat = dir.join(format!("{stem}.yali-{}.flat.png", std::process::id()));
    let plan = crate::engines::registry::SubprocessPlan {
        program: vips_cli,
        args: vec![
            "flatten".into(),
            input.to_string_lossy().into(),
            flat.to_string_lossy().into(),
            "--background".into(),
            "65535".into(),
        ],
    };
    if run_plan(&plan).await.is_err() || !flat.exists() {
        let _ = std::fs::remove_file(&flat);
        return Ok(None);
    }
    // 结构校验（零成本）：flatten 产物必须是 RGB。libvips 对无 alpha 的调色板 PNG
    // 会输出 graya 灰白图（实测毁图），这里当场判失败让调用方回落 sips。
    if png_color_type(&flat) != Some(2) {
        let _ = std::fs::remove_file(&flat);
        return Ok(None);
    }
    Ok(Some(flat))
}

/// 容器解码：macOS 用 sips，其他平台用 libvips 解成 PNG。
async fn decode_to_png_via_sips(
    input: &Path,
    dir: &Path,
    registry: &Registry,
) -> Result<PathBuf> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("img");
    let png = dir.join(format!("{stem}.yali-{}.decode.png", std::process::id()));
    #[cfg(target_os = "macos")]
    let plan = crate::engines::registry::SubprocessPlan {
        program: registry
            .path_of(crate::engines::EngineKind::Sips)
            .unwrap_or_else(|| PathBuf::from("/usr/bin/sips")),
        args: vec![
            "-s".into(),
            "format".into(),
            "png".into(),
            input.to_string_lossy().into(),
            "--out".into(),
            png.to_string_lossy().into(),
        ],
    };
    #[cfg(not(target_os = "macos"))]
    let plan = crate::engines::registry::SubprocessPlan {
        program: crate::engines::EngineKind::Vips
            .sibling_binary("vips")
            .context("当前平台缺少 libvips，无法解码容器图片")?,
        args: vec!["copy".into(), input.to_string_lossy().into(), png.to_string_lossy().into()],
    };
    run_plan(&plan).await?;
    if !png.exists() {
        bail!("sips 容器解码失败: {}", input.display());
    }
    Ok(png)
}

/// 像素尺寸探测（macOS 用 sips，其他平台用 vipsheader；失败返回 None）。
async fn probe_dims(path: &Path) -> Option<(u32, u32)> {
    #[cfg(target_os = "macos")]
    let mut command = tokio::process::Command::new("sips");
    #[cfg(target_os = "macos")]
    command.args(["-g", "pixelWidth", "-g", "pixelHeight"]);
    #[cfg(not(target_os = "macos"))]
    let mut command = tokio::process::Command::new(
        crate::runtime::bundled_binary("vipsheader").unwrap_or_else(|| PathBuf::from("vipsheader")),
    );
    #[cfg(not(target_os = "macos"))]
    command.arg("-a");
    let out = command
        .arg(path)
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut w = None;
    let mut h = None;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("pixelWidth:").or_else(|| line.trim().strip_prefix("width:")) {
            w = v.trim().parse().ok();
        }
        if let Some(v) = line.trim().strip_prefix("pixelHeight:").or_else(|| line.trim().strip_prefix("height:")) {
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
    #[test]
    fn windows_limits_supported_input_formats() {
        use super::{input_supported, InputFormat};
        for format in [InputFormat::Jpeg, InputFormat::Png, InputFormat::Gif, InputFormat::Webp, InputFormat::Bmp, InputFormat::Tiff] { assert!(input_supported(format, true)); }
        for format in [InputFormat::Svg, InputFormat::Avif, InputFormat::Heic, InputFormat::Jxl] { assert!(!input_supported(format, true)); assert!(input_supported(format, false)); }
    }

    use super::*;
    use crate::profile::DirKind;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct TestHome(PathBuf, Option<std::ffi::OsString>);
    impl TestHome {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("yali-recommended-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
            let previous = std::env::var_os("YALI_HOME");
            std::env::set_var("YALI_HOME", &root);
            Self(root, previous)
        }
    }
    impl Drop for TestHome {
        fn drop(&mut self) {
            match &self.1 {
                Some(value) => std::env::set_var("YALI_HOME", value),
                None => std::env::remove_var("YALI_HOME"),
            }
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn recommended_handles_quality_rejection_and_optional_optimizer_failure() {
        use std::os::unix::fs::PermissionsExt;
        let _lock = crate::test_support::ENV_LOCK.lock().unwrap();
        let _home = TestHome::new();
        let dir = crate::data::TempDir::new().unwrap();
        let mut profile: Profile = serde_json::from_str(include_str!("../../../fixtures/default.json")).unwrap();
        profile.output.undo_manifest = false;
        let input = dir.path().join("ui.png");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchkit/samples/ui.png"), &input).unwrap();
        let original = std::fs::read(&input).unwrap();
        struct Override(Option<std::ffi::OsString>);
        impl Drop for Override {
            fn drop(&mut self) {
                match &self.0 {
                    Some(value) => std::env::set_var("YALI_ENGINES_PATH", value),
                    None => std::env::remove_var("YALI_ENGINES_PATH"),
                }
            }
        }
        let _restore = Override(std::env::var_os("YALI_ENGINES_PATH"));
        std::env::set_var("YALI_ENGINES_PATH", dir.path());
        let script = |name: &str, content: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, content).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        };
        script("pngquant", "#!/bin/sh\nexit 99\n");
        let report = compress_file(&input, &profile, &Registry::discover(&profile)).await.unwrap();
        assert_eq!(report.engine, "oxipng", "质量未达标只能回退无损");
        script("pngquant", "#!/bin/sh\nexit 1\n");
        assert!(compress_file(&input, &profile, &Registry::discover(&profile)).await.is_err(), "普通引擎错误不能伪装为质量回退");
        std::fs::remove_file(dir.path().join("pngquant")).unwrap();
        script("oxipng", "#!/bin/sh\nexit 1\n");
        let report = compress_file(&input, &profile, &Registry::discover(&profile)).await.unwrap();
        assert_eq!(report.engine, "pngquant");
        assert_eq!(report.status, "done", "可选后处理失败应保留有效量化结果");
        assert_eq!(std::fs::read(&input).unwrap(), original);
    }

    #[tokio::test]
    async fn recommended_rejects_invalid_and_larger_candidates() {
        let _lock = crate::test_support::ENV_LOCK.lock().unwrap();
        let _home = TestHome::new();
        let dir = crate::data::TempDir::new().unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchkit/samples/photo.jpg");
        let output = dir.path().join("primary.jpg");
        let candidate = dir.path().join("candidate.jpg");
        std::fs::copy(&input, &output).unwrap();
        let original = std::fs::read(&output).unwrap();
        std::fs::write(&candidate, b"invalid").unwrap();
        assert!(!choose_smaller_candidate(&input, &output, &candidate, crate::profile::OutputFormat::Jpeg).await.unwrap());
        std::fs::write(&candidate, vec![0; original.len() + 1]).unwrap();
        assert!(!choose_smaller_candidate(&input, &output, &candidate, crate::profile::OutputFormat::Jpeg).await.unwrap());
        assert_eq!(std::fs::read(output).unwrap(), original);
    }

    #[tokio::test]
    async fn recommended_thumbnail_and_already_compressed_jpeg_preserve_user_intent() {
        let _lock = crate::test_support::ENV_LOCK.lock().unwrap();
        let _home = TestHome::new();
        let dir = crate::data::TempDir::new().unwrap();
        let mut profile: Profile = serde_json::from_str(include_str!("../../../fixtures/thumbnail.json")).unwrap();
        profile.output.undo_manifest = false;
        let input = dir.path().join("photo.jpg");
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchkit/samples/photo.jpg"), &input).unwrap();
        let registry = Registry::discover(&profile);
        let report = compress_file(&input, &profile, &registry).await.unwrap();
        assert_eq!(probe_dims(Path::new(report.output.as_ref().unwrap())).await, Some((1024, 683)), "不能让未缩放的无损候选覆盖缩放结果");
        let low = dir.path().join("low.jpg");
        run_plan(&registry.encoder_of(EngineKind::Mozjpeg).unwrap().plan(&EncodeRequest {
            input, output: low.clone(), input_fmt: InputFormat::Jpeg,
            output_fmt: crate::profile::OutputFormat::Jpeg, level: Some(10), lossless: false, recommended: false,
        }).unwrap()).await.unwrap();
        profile.resize.max_width = None;
        profile.resize.max_height = None;
        let original = std::fs::read(&low).unwrap();
        let report = compress_file(&low, &profile, &registry).await.unwrap();
        assert_eq!(std::fs::read(&low).unwrap(), original);
        if report.status == "done" {
            assert!(report.output_bytes.unwrap() < report.input_bytes);
        } else {
            assert_eq!(report.status, "larger_output");
            assert!(report.output.is_none());
        }
    }

    #[tokio::test]
    async fn thumbnail_shrinks_large_images_but_never_enlarges_small_ones() {
        let mut profile: Profile = serde_json::from_str(include_str!("../../../fixtures/thumbnail.json")).unwrap();
        let registry = Registry::discover(&profile);
        let dir = std::env::temp_dir().join(format!("yali-thumbnail-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchkit/samples/photo.jpg");
        let first = pre_scaled_copy(&input, InputFormat::Jpeg, &profile, &registry, &dir).await.unwrap().unwrap();
        assert_eq!(probe_dims(&first).await, Some((1024, 683)));
        profile.resize.max_width = Some(2048);
        profile.resize.max_height = Some(2048);
        let second = pre_scaled_copy(&first, InputFormat::Png, &profile, &registry, &dir).await.unwrap().unwrap();
        assert_eq!(probe_dims(&second).await, Some((1024, 683)), "shrinkOnly 不得放大小图");
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// 造一个只含 chunk 结构的假 PNG 头（解析只看结构，不校验 CRC）
    fn fake_png(color_type: u8, with_trns: bool) -> Vec<u8> {
        let mut out: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // IHDR: width(4) height(4) bit_depth(1) color_type(1) compress(1) filter(1) interlace(1)
        let mut ihdr = vec![0u8; 13];
        ihdr[0..4].copy_from_slice(&[0, 0, 0, 2]); // width
        ihdr[4..8].copy_from_slice(&[0, 0, 0, 2]); // height
        ihdr[8] = 8; // bit depth
        ihdr[9] = color_type;
        out.extend_from_slice(&[0, 0, 0, 13]);
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&ihdr);
        out.extend_from_slice(&[0u8; 4]); // crc
        if with_trns {
            out.extend_from_slice(&[0, 0, 0, 2]);
            out.extend_from_slice(b"tRNS");
            out.extend_from_slice(&[255, 0]);
            out.extend_from_slice(&[0u8; 4]);
        }
        out.extend_from_slice(&[0, 0, 0, 1]);
        out.extend_from_slice(b"IDAT");
        out.extend_from_slice(&[0]);
        out.extend_from_slice(&[0u8; 4]);
        out
    }

    fn write_fake(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("yali-pnghdr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn pipeline_palette_png_chooses_lossless_engine() {
        use crate::engines::EngineKind;
        assert_eq!(png_png_engine_choice(Some(3)), EngineKind::Oxipng, "调色板 PNG 不该二次量化");
        assert_eq!(png_png_engine_choice(Some(2)), EngineKind::Mozjpeg);
        assert_eq!(png_png_engine_choice(Some(6)), EngineKind::Mozjpeg);
        assert_eq!(png_png_engine_choice(None), EngineKind::Mozjpeg);
    }

    #[test]
    fn pipeline_png_alpha_detection() {
        // type 6 RGBA / type 4 灰度+alpha → 有透明
        assert!(png_has_alpha(&write_fake("t6.png", &fake_png(6, false))));
        assert!(png_has_alpha(&write_fake("t4.png", &fake_png(4, false))));
        // type 3 调色板：有 tRNS 才算透明；没有就不该 flatten
        assert!(png_has_alpha(&write_fake("t3trns.png", &fake_png(3, true))));
        assert!(!png_has_alpha(&write_fake("t3.png", &fake_png(3, false))));
        // type 2 RGB / type 0 灰度 → 无透明
        assert!(!png_has_alpha(&write_fake("t2.png", &fake_png(2, false))));
        assert!(!png_has_alpha(&write_fake("t0.png", &fake_png(0, false))));
        assert_eq!(png_color_type(&write_fake("t2b.png", &fake_png(2, false))), Some(2));
        assert_eq!(png_color_type(Path::new("/nonexistent.png")), None);
    }

    #[test]
    fn pipeline_sniff_common_formats() {
        let dir = std::env::temp_dir().join(format!("yali-sniff-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cases: Vec<(&[u8], InputFormat)> = vec![
            (&[0xFF, 0xD8, 0xFF, 0xE0], InputFormat::Jpeg),
            (&[0x89, b'P', b'N', b'G', 0x0D, 0x0A], InputFormat::Png),
            (b"GIF89a", InputFormat::Gif),
            (b"BM\x00\x00\x00\x00", InputFormat::Bmp),
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
        std::env::set_var("HOME", "/tmp/yali-test-home");
        std::env::remove_var("YALI_OUTPUT_DIR");

        let input = Path::new("/tmp/photo/photo.jpg");
        let mut profile = Profile::default();
        profile.name = "t".into();

        // sibling：与输入同目录
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/photo"));

        // downloads：$HOME/Downloads
        profile.output.dir = DirKind::Downloads;
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/yali-test-home/Downloads"));

        // custom：绝对路径 / ~ 展开
        profile.output.dir = DirKind::Custom;
        profile.output.dir_path = Some("/tmp/yali-test-out".into());
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/yali-test-out"));
        profile.output.dir_path = Some("~/压缩输出".into());
        assert_eq!(output_dir(input, &profile).unwrap(), PathBuf::from("/tmp/yali-test-home/压缩输出"));

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
