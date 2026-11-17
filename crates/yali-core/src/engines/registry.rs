//! EngineRegistry：按输出格式注册编码器；`pipeline.engine.<fmt>` 可覆盖。
//! 所有引擎都是子进程（Phase A），本模块只做「计划」——拼参数与程序路径，
//! 执行统一走 [`run_plan`]，负优化判定 / 命名 / undo 在 pipeline。

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Result};

use super::mapping;
use super::{EngineKind, EngineProbe};
use crate::profile::{OutputFormat, Profile};

/// 魔数嗅探出的真实输入格式（pipeline.rs 定义，此处仅引用）
use crate::pipeline::InputFormat;

/// 一次编码请求：level 为 Some 时走有损映射，lossless 为 true 时走无损路径
#[derive(Debug, Clone)]
pub struct EncodeRequest {
    pub input: PathBuf,
    pub output: PathBuf,
    pub input_fmt: InputFormat,
    pub output_fmt: OutputFormat,
    pub level: Option<u8>,
    pub lossless: bool,
}

/// 子进程执行计划
#[derive(Debug, Clone)]
pub struct SubprocessPlan {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// 编码器：只负责把请求翻译成子进程参数（DESIGN §7 Encoder trait 的 Phase A 形态）
pub trait Encoder: Send + Sync {
    fn name(&self) -> &'static str;
    fn kind(&self) -> EngineKind;
    /// 是否处理该（输入格式 → 输出格式）组合
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool;
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan>;
}

fn need_level(req: &EncodeRequest) -> Result<u8> {
    req.level.ok_or_else(|| anyhow::anyhow!("该引擎需要 level 有损档"))
}

// ---------------- 各引擎实现 ----------------

/// mozjpeg：JPEG 有损主力（cjpeg）+ JPEG 无损重压（jpegtran，同一 keg 路径）
struct MozjpegEncoder {
    cjpeg: PathBuf,
    jpegtran: Option<PathBuf>,
}

impl Encoder for MozjpegEncoder {
    fn name(&self) -> &'static str {
        "mozjpeg"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Mozjpeg
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        output == OutputFormat::Jpeg && matches!(input, InputFormat::Jpeg | InputFormat::Png)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        if req.lossless {
            let program = self.jpegtran.as_ref().ok_or_else(|| {
                anyhow::anyhow!("mozjpeg jpegtran 不可用（无损重压需要它）")
            })?;
            return Ok(SubprocessPlan {
                program: program.clone(),
                args: vec![
                    "-copy".into(), "all".into(),
                    "-optimize".into(),
                    "-outfile".into(), req.output.to_string_lossy().into(),
                    req.input.to_string_lossy().into(),
                ],
            });
        }
        let q = mapping::jpeg_quality(need_level(req)?);
        Ok(SubprocessPlan {
            program: self.cjpeg.clone(),
            args: vec![
                "-quality".into(), q.to_string(),
                "-outfile".into(), req.output.to_string_lossy().into(),
                req.input.to_string_lossy().into(),
            ],
        })
    }
}

/// cwebp：WebP 输出（jpeg/png/webp 输入直接吃）
struct CwebpEncoder {
    cwebp: PathBuf,
}

impl Encoder for CwebpEncoder {
    fn name(&self) -> &'static str {
        "cwebp"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Cwebp
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        output == OutputFormat::Webp && matches!(input, InputFormat::Jpeg | InputFormat::Png | InputFormat::Webp)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        let mut args = Vec::new();
        if req.lossless {
            args.push("-z".into());
            args.push("9".into()); // 无损档：最高压缩
        } else {
            args.push("-q".into());
            args.push(mapping::webp_quality(need_level(req)?).to_string());
        }
        args.push(req.input.to_string_lossy().into());
        args.push("-o".into());
        args.push(req.output.to_string_lossy().into());
        Ok(SubprocessPlan { program: self.cwebp.clone(), args })
    }
}

/// gif2webp：动图转 WebP（实测 -24%，AnimatedPolicy::webp 显式开启）
struct Gif2WebpEncoder {
    gif2webp: PathBuf,
}

impl Encoder for Gif2WebpEncoder {
    fn name(&self) -> &'static str {
        "gif2webp"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Gif2Webp
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input == InputFormat::Gif && output == OutputFormat::Webp
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        let q = mapping::webp_quality(need_level(req)?);
        Ok(SubprocessPlan {
            program: self.gif2webp.clone(),
            args: vec!["-q".into(), q.to_string(), req.input.to_string_lossy().into(), "-o".into(), req.output.to_string_lossy().into()],
        })
    }
}

/// avifenc：AVIF 输出（速度档 v1 固定 -s 6）
struct AvifencEncoder {
    avifenc: PathBuf,
}

impl Encoder for AvifencEncoder {
    fn name(&self) -> &'static str {
        "avifenc"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Avifenc
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        output == OutputFormat::Avif && matches!(input, InputFormat::Jpeg | InputFormat::Png | InputFormat::Gif)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        if req.lossless {
            bail!("avifenc 暂不支持无损档（M3 任务）");
        }
        let q = mapping::webp_quality(need_level(req)?);
        Ok(SubprocessPlan {
            program: self.avifenc.clone(),
            args: vec!["-q".into(), q.to_string(), "-s".into(), "6".into(), req.input.to_string_lossy().into(), req.output.to_string_lossy().into()],
        })
    }
}

/// pngquant：PNG 有损量化（level 3-10）
struct PngquantEncoder {
    pngquant: PathBuf,
}

impl Encoder for PngquantEncoder {
    fn name(&self) -> &'static str {
        "pngquant"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Pngquant
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input == InputFormat::Png && output == OutputFormat::Png
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        if req.lossless {
            bail!("pngquant 是有损引擎，无损请走 oxipng");
        }
        let level = need_level(req)?;
        if level < 3 {
            bail!("level {level} 属于无损档，应走 oxipng");
        }
        let range = format!(
            "{}-{}",
            mapping::pngquant_quality_min(level),
            mapping::pngquant_quality_max(level)
        );
        Ok(SubprocessPlan {
            program: self.pngquant.clone(),
            args: vec![
                "--quality".into(), range,
                "--speed".into(), mapping::pngquant_speed(level).to_string(),
                "--force".into(),
                "--output".into(), req.output.to_string_lossy().into(),
                req.input.to_string_lossy().into(),
            ],
        })
    }
}

/// oxipng：PNG 无损（level 1-2 与 lossless 档共用）
struct OxipngEncoder {
    oxipng: PathBuf,
}

impl Encoder for OxipngEncoder {
    fn name(&self) -> &'static str {
        "oxipng"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Oxipng
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input == InputFormat::Png && output == OutputFormat::Png
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        let level = if req.lossless { 2 } else { need_level(req)? };
        if !req.lossless && level >= 3 {
            bail!("level {level} 属于有损档，应走 pngquant");
        }
        Ok(SubprocessPlan {
            program: self.oxipng.clone(),
            args: vec![
                "-o".into(), mapping::oxipng_optimization(level).to_string(),
                "--strip".into(), "safe".into(),
                "--out".into(), req.output.to_string_lossy().into(),
                req.input.to_string_lossy().into(),
            ],
        })
    }
}

/// gifsicle：GIF（-O3 无损优化；--colors 仅 level 有损档）
struct GifsicleEncoder {
    gifsicle: PathBuf,
}

impl Encoder for GifsicleEncoder {
    fn name(&self) -> &'static str {
        "gifsicle"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Gifsicle
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input == InputFormat::Gif && output == OutputFormat::Auto || input == InputFormat::Gif && output == OutputFormat::Gif
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        let mut args = vec!["-O3".into()];
        if !req.lossless {
            args.push("--colors".into());
            args.push(mapping::gif_colors(need_level(req)?).to_string());
        }
        args.push("-o".into());
        args.push(req.output.to_string_lossy().into());
        args.push(req.input.to_string_lossy().into());
        Ok(SubprocessPlan { program: self.gifsicle.clone(), args })
    }
}

/// cjxl：JXL（jpeg 无损包裹可逐比特还原；png 走 -d 0）
struct CjxlEncoder {
    cjxl: PathBuf,
}

impl Encoder for CjxlEncoder {
    fn name(&self) -> &'static str {
        "cjxl"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Cjxl
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        output == OutputFormat::Jxl && matches!(input, InputFormat::Jpeg | InputFormat::Png)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        let mut args = Vec::new();
        if req.lossless || req.input_fmt == InputFormat::Jpeg {
            // jpeg 无损包裹（--lossless_jpeg=1），djxl 逐比特还原
            args.push("--lossless_jpeg=1".into());
        } else {
            args.push("-d".into());
            args.push(mapping::jxl_distance(need_level(req)?).to_string());
        }
        args.push(req.input.to_string_lossy().into());
        args.push(req.output.to_string_lossy().into());
        Ok(SubprocessPlan { program: self.cjxl.clone(), args })
    }
}

/// svgo：SVG（node 安装，本机可能缺席；缺席时报错不阻塞其他格式）
struct SvgoEncoder {
    svgo: PathBuf,
}

impl Encoder for SvgoEncoder {
    fn name(&self) -> &'static str {
        "svgo"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Svgo
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input == InputFormat::Svg && (output == OutputFormat::Svg || output == OutputFormat::Auto)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        Ok(SubprocessPlan {
            program: self.svgo.clone(),
            args: vec!["-i".into(), req.input.to_string_lossy().into(), "-o".into(), req.output.to_string_lossy().into()],
        })
    }
}

/// sips：macOS 原生（HEIC 硬编，实测 112ms）
struct SipsEncoder {
    sips: PathBuf,
}

impl Encoder for SipsEncoder {
    fn name(&self) -> &'static str {
        "sips"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Sips
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        output == OutputFormat::Heic && matches!(input, InputFormat::Jpeg | InputFormat::Png | InputFormat::Tiff | InputFormat::Heic)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        if req.lossless {
            bail!("HEIC 无损未定义（M3 任务）");
        }
        let q = mapping::webp_quality(need_level(req)?);
        Ok(SubprocessPlan {
            program: self.sips.clone(),
            args: vec![
                "-s".into(), "format".into(), "heic".into(),
                "-s".into(), "formatOptions".into(), q.to_string(),
                req.input.to_string_lossy().into(),
                "--out".into(), req.output.to_string_lossy().into(),
            ],
        })
    }
}

/// vipsthumbnail：兜底转换器（不直接吃 SVG/不认识的组合先转 PNG）+ 预缩放
struct VipsEncoder {
    vips: PathBuf,
}

impl Encoder for VipsEncoder {
    fn name(&self) -> &'static str {
        "vips"
    }
    fn kind(&self) -> EngineKind {
        EngineKind::Vips
    }
    fn supports(&self, input: InputFormat, output: OutputFormat) -> bool {
        input != InputFormat::Svg
            && !matches!(output, OutputFormat::Auto | OutputFormat::Svg)
    }
    fn plan(&self, req: &EncodeRequest) -> Result<SubprocessPlan> {
        if req.lossless {
            bail!("vips 兜底转换不支持无损档（无损组合请用专用引擎）");
        }
        let q = mapping::webp_quality(need_level(req)?);
        let ext = match req.output_fmt {
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
            OutputFormat::Webp => "webp",
            OutputFormat::Avif => "avif",
            OutputFormat::Heic => "heic",
            OutputFormat::Jxl => "jxl",
            OutputFormat::Auto | OutputFormat::Gif | OutputFormat::Svg => {
                bail!("vips 兜底不处理 {:?}/{:?}", req.input_fmt, req.output_fmt)
            }
        };
        // vipsthumbnail 按输出扩展名定目标格式；质量用文件名选项语法 out.jpg[Q=80]
        let out_path = req.output.with_extension(ext);
        let out = format!("{}[Q={q}]", out_path.to_string_lossy());
        Ok(SubprocessPlan {
            program: self.vips.clone(),
            args: vec![req.input.to_string_lossy().into(), "-o".into(), out],
        })
    }
}

// ---------------- Registry ----------------

/// 引擎注册表：路径解析一次、全批复用；按（输入→输出）选编码器
pub struct Registry {
    pub probes: HashMap<EngineKind, EngineProbe>,
    encoders: Vec<Box<dyn Encoder>>,
}

impl Registry {
    /// 探测所有引擎并构建编码器集合；jpeg 覆盖项来自 profile.pipeline.engine.jpeg
    pub fn discover(_profile: &Profile) -> Self {
        let kinds = [
            EngineKind::Mozjpeg,
            EngineKind::Pngquant,
            EngineKind::Oxipng,
            EngineKind::Gifsicle,
            EngineKind::Gif2Webp,
            EngineKind::Cwebp,
            EngineKind::Avifenc,
            EngineKind::Cjxl,
            EngineKind::Svgo,
            EngineKind::Vips,
            EngineKind::Sips,
        ];
        let probes: HashMap<_, _> = kinds.iter().map(|k| (*k, k.probe())).collect();
        let path_of = |k: EngineKind| probes.get(&k).and_then(|p| p.path.clone());

        let mut encoders: Vec<Box<dyn Encoder>> = Vec::new();
        if let Some(cjpeg) = path_of(EngineKind::Mozjpeg) {
            let jpegtran = EngineKind::Mozjpeg.sibling_binary("jpegtran");
            encoders.push(Box::new(MozjpegEncoder { cjpeg, jpegtran }));
        }
        if let Some(cwebp) = path_of(EngineKind::Cwebp) {
            encoders.push(Box::new(CwebpEncoder { cwebp }));
        }
        if let Some(gif2webp) = path_of(EngineKind::Gif2Webp) {
            encoders.push(Box::new(Gif2WebpEncoder { gif2webp }));
        }
        if let Some(avifenc) = path_of(EngineKind::Avifenc) {
            encoders.push(Box::new(AvifencEncoder { avifenc }));
        }
        if let Some(pngquant) = path_of(EngineKind::Pngquant) {
            encoders.push(Box::new(PngquantEncoder { pngquant }));
        }
        if let Some(oxipng) = path_of(EngineKind::Oxipng) {
            encoders.push(Box::new(OxipngEncoder { oxipng }));
        }
        if let Some(gifsicle) = path_of(EngineKind::Gifsicle) {
            encoders.push(Box::new(GifsicleEncoder { gifsicle }));
        }
        if let Some(cjxl) = path_of(EngineKind::Cjxl) {
            encoders.push(Box::new(CjxlEncoder { cjxl }));
        }
        if let Some(svgo) = path_of(EngineKind::Svgo) {
            encoders.push(Box::new(SvgoEncoder { svgo }));
        }
        if let Some(sips) = path_of(EngineKind::Sips) {
            encoders.push(Box::new(SipsEncoder { sips }));
        }
        if let Some(vips) = path_of(EngineKind::Vips) {
            encoders.push(Box::new(VipsEncoder { vips }));
        }
        // vips 兜底必须排最后：专用引擎优先
        Self { probes, encoders }
    }

    /// 选编码器（专用引擎优先，vips 兜底最后）
    pub fn pick(
        &self,
        input: InputFormat,
        output: OutputFormat,
    ) -> Result<&dyn Encoder> {
        let output = if output == OutputFormat::Auto { Self::auto_of(input) } else { output };
        self.encoders
            .iter()
            .find(|e| e.supports(input, output))
            .map(|e| e.as_ref())
            .ok_or_else(|| anyhow::anyhow!("没有引擎支持 {input:?} → {output:?}（doctor 查看可用性）"))
    }

    /// auto 路由：保持原格式（TIFF 例外：输出枚举无 TIFF，落到 JPEG）
    pub fn auto_of(input: InputFormat) -> OutputFormat {
        match input {
            InputFormat::Jpeg => OutputFormat::Jpeg,
            InputFormat::Png => OutputFormat::Png,
            InputFormat::Gif => OutputFormat::Gif,
            InputFormat::Webp => OutputFormat::Webp,
            InputFormat::Svg => OutputFormat::Svg,
            InputFormat::Tiff => OutputFormat::Jpeg,
            InputFormat::Avif => OutputFormat::Avif,
            InputFormat::Heic => OutputFormat::Heic,
            InputFormat::Jxl => OutputFormat::Jxl,
        }
    }

    pub fn path_of(&self, kind: EngineKind) -> Option<PathBuf> {
        self.probes.get(&kind).and_then(|p| p.path.clone())
    }

    /// 进程池预热：并发跑一遍各引擎 --version，把二进制与动态库压进 page cache。
    /// libvips 冷启动 ~5s，这是设计要求而非优化项（docs/DESIGN.md §3 / 坑账本）。
    pub async fn warmup(&self) {
        let tasks: Vec<_> = self
            .probes
            .values()
            .filter_map(|p| p.path.clone())
            .map(|path| tokio::spawn(async move {
                let _ = tokio::process::Command::new(&path).arg("--version").output().await;
            }))
            .collect();
        for t in tasks {
            let _ = t.await;
        }
    }
}

/// 执行子进程计划；非零退出报 stderr 尾部
pub async fn run_plan(plan: &SubprocessPlan) -> Result<()> {
    let out = super::run_engine(&plan.program, &plan.args).await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tail: String = stderr.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
        bail!(
            "引擎 {} 退出码 {:?}: {}",
            plan.program.display(),
            out.status.code(),
            tail
        );
    }
    Ok(())
}
