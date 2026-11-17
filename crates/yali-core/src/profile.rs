//! Profile：所有入口（右键/GUI/CLI）的唯一参数事实源。
//! M-1 占位版：仅核心字段 + serde 往返；完整 Schema 见 docs/DESIGN.md §2（T-002 落定）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QualityKind {
    #[default]
    Level,
    #[serde(rename = "targetBytes")]
    TargetBytes,
    Lossless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Auto,
    Jpeg,
    Png,
    Webp,
    Avif,
    Heic,
    Jxl,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExifPolicy {
    Keep,
    #[default]
    Strip,
    Privacy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub level: Option<u8>, // 1..=10
    pub target_bytes: Option<u64>,
    pub quality_kind: QualityKind,
    pub output_format: OutputFormat,
    pub dir_kind: DirKind,
    pub suffix_template: String,
    pub exif: ExifPolicy,
}

impl Profile {
    /// 出厂默认档「压一下」：参数依据 docs/BENCHMARKS.md 实测标定（level 4）
    pub fn default_profile() -> Self {
        Self {
            name: "default".into(),
            level: Some(4),
            target_bytes: None,
            quality_kind: QualityKind::Level,
            output_format: OutputFormat::Auto,
            dir_kind: DirKind::Sibling,
            suffix_template: "-{profile}".into(),
            exif: ExifPolicy::Privacy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_serde_roundtrip() {
        let p = Profile::default_profile();
        let json = serde_json::to_string(&p).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "default");
        assert_eq!(back.level, Some(4));
        assert_eq!(back.exif, ExifPolicy::Privacy);
    }

    #[test]
    fn camel_case_field_names() {
        let json = serde_json::to_string(&Profile::default_profile()).unwrap();
        assert!(json.contains("targetBytes"), "camelCase 约定: {json}");
        assert!(json.contains("suffixTemplate"));
    }
}
