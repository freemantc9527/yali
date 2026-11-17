//! Profile：所有入口（右键/GUI/CLI）的唯一参数事实源。
//! 字段权威定义：docs/DESIGN.md §2；JSON Schema（文档事实源）：schema/profile.schema.json。
//!
//! 序列化约定：camelCase + `deny_unknown_fields`（未知字段报错并指出字段名）。
//! Rust 侧靠 serde 强类型校验，不引 jsonschema crate。

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

// ---------------- 枚举 ----------------

/// quality.kind 三态：level(1-10) / targetBytes / lossless
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QualityKind {
    #[default]
    Level,
    #[serde(rename = "targetBytes")]
    TargetBytes,
    Lossless,
}

/// format.output：auto 表示按输入格式走默认引擎路由。
/// Gif/Svg 仅作为管道内部路由目标（Profile JSON 不会出现，schema 不含此二值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Auto,
    Jpeg,
    Png,
    Gif,
    Webp,
    Svg,
    Avif,
    Heic,
    Jxl,
}

impl OutputFormat {
    pub fn ext(self) -> &'static str {
        match self {
            OutputFormat::Auto => "auto",
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
            OutputFormat::Gif => "gif",
            OutputFormat::Webp => "webp",
            OutputFormat::Svg => "svg",
            OutputFormat::Avif => "avif",
            OutputFormat::Heic => "heic",
            OutputFormat::Jxl => "jxl",
        }
    }
}

/// 动图策略：默认保守（保持 GIF）；webp 显式开启（实测 -24%，docs/BENCHMARKS.md）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnimatedPolicy {
    #[default]
    Keep,
    Webp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExifPolicy {
    Keep,
    Strip,
    #[default]
    Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IccPolicy {
    #[default]
    Srgb,
    Keep,
    Strip,
}

/// JPEG 引擎选择（brew 的 cjpeg 是 libjpeg-turbo 李鬼，必须走 mozjpeg 包路径——坑账本）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JpegEngine {
    #[default]
    Mozjpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ResizeMode {
    #[default]
    ShrinkOnly,
    Always,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CropAnchor {
    #[default]
    Center,
    Smart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DirKind {
    #[default]
    Sibling,
    Override,
    Custom,
    Downloads,
}

/// 负优化拦截：输出 ≥ 输入 → 不落盘，状态 larger_output，UI 标 ⚠（DESIGN §3）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LargerPolicy {
    #[default]
    KeepOriginal,
    SaveLarger,
}

// ---------------- 六段结构 ----------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Pipeline {
    /// 内容分类路由（M3 起启用）：照片→有损管道 / UI 截图→无损 WebP / 透明→pngquant 或 oxipng
    pub classify: bool,
    pub engine: Engines,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self { classify: true, engine: Engines::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Engines {
    pub jpeg: JpegEngine,
}

impl Default for Engines {
    fn default() -> Self {
        Self { jpeg: JpegEngine::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Resize {
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub keep_ratio: bool,
    pub mode: ResizeMode,
    pub crop_anchor: Option<CropAnchor>,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            max_width: None,
            max_height: None,
            keep_ratio: true,
            mode: ResizeMode::default(),
            crop_anchor: None,
        }
    }
}

/// `minQualityFloor` 只在 targetBytes 下生效（二分下探的保护线）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Quality {
    pub kind: QualityKind,
    pub level: Option<u8>,
    pub target_bytes: Option<u64>,
    pub min_quality_floor: u8,
}

impl Default for Quality {
    fn default() -> Self {
        Self { kind: QualityKind::default(), level: None, target_bytes: None, min_quality_floor: 3 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Format {
    pub output: OutputFormat,
    pub animated: AnimatedPolicy,
}

impl Default for Format {
    fn default() -> Self {
        Self { output: OutputFormat::default(), animated: AnimatedPolicy::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Metadata {
    pub exif: ExifPolicy,
    pub icc: IccPolicy,
}

impl Default for Metadata {
    fn default() -> Self {
        Self { exif: ExifPolicy::default(), icc: IccPolicy::default() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Output {
    pub dir: DirKind,
    /// 支持 {profile} / {date} / {size} 占位符
    pub suffix_template: String,
    pub larger_policy: LargerPolicy,
    pub undo_manifest: bool,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            dir: DirKind::default(),
            suffix_template: "-{profile}".into(),
            larger_policy: LargerPolicy::default(),
            undo_manifest: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Ui {
    pub notify: bool,
    pub sound: bool,
    #[serde(rename = "openGUIOnFail")]
    pub open_gui_on_fail: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Self { notify: true, sound: false, open_gui_on_fail: true }
    }
}

// ---------------- Profile 根 ----------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub pipeline: Pipeline,
    pub resize: Resize,
    pub quality: Quality,
    pub format: Format,
    pub metadata: Metadata,
    pub output: Output,
    pub ui: Ui,
}

impl Profile {
    /// 语义校验（serde 管类型/未知字段，这里管取值范围与字段联动）
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("profile.name 不能为空");
        }
        if let Some(level) = self.quality.level {
            if !(1..=10).contains(&level) {
                bail!("profile '{}': quality.level 必须在 1..=10，得到 {level}", self.name);
            }
        }
        match self.quality.kind {
            QualityKind::Level if self.quality.level.is_none() => {
                bail!("profile '{}': quality.kind=level 需要 quality.level", self.name);
            }
            QualityKind::TargetBytes => {
                let tb = self
                    .quality
                    .target_bytes
                    .ok_or_else(|| anyhow::anyhow!("profile '{}': quality.kind=targetBytes 需要 quality.targetBytes", self.name))?;
                if tb == 0 {
                    bail!("profile '{}': quality.targetBytes 必须大于 0", self.name);
                }
            }
            _ => {}
        }
        if self.output.suffix_template.is_empty() {
            bail!("profile '{}': output.suffixTemplate 不能为空", self.name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_camel_case_field_names() {
        let p = Profile::default();
        let json = serde_json::to_string(&p).unwrap();
        for key in ["targetBytes", "minQualityFloor", "suffixTemplate", "openGUIOnFail", "keepRatio", "maxWidth"] {
            assert!(json.contains(&format!("\"{key}\"")), "camelCase 约定缺 {key}: {json}");
        }
    }

    #[test]
    fn profile_rejects_unknown_field_with_name() {
        let err = serde_json::from_str::<Profile>(r#"{"name":"x","bogusField":1}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bogusField"), "报错应指出未知字段名: {msg}");
    }

    #[test]
    fn profile_invalid_json_error_contains_line_number() {
        let err = serde_json::from_str::<Profile>("{\n  \"name\": \"x\",\n  \"broken").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("line"), "非法 JSON 报错应含行号: {msg}");
    }

    #[test]
    fn profile_validate_level_range() {
        let mut p = Profile::default();
        p.name = "t".into();
        p.quality.kind = QualityKind::Level;
        p.quality.level = Some(0);
        assert!(p.validate().is_err());
        p.quality.level = Some(11);
        assert!(p.validate().is_err());
        p.quality.level = Some(4);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn profile_validate_kind_field_binding() {
        let mut p = Profile::default();
        p.name = "t".into();
        p.quality.kind = QualityKind::TargetBytes;
        assert!(p.validate().is_err(), "targetBytes 档缺 targetBytes 应报错");
        p.quality.target_bytes = Some(500_000);
        assert!(p.validate().is_ok());
    }
}
