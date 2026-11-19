//! Yali 内核：Profile / Store / Pipeline / EngineRegistry / UndoManifest（无 UI 依赖）。
//!
//! 设计权威：docs/DESIGN.md；Profile 字段：docs/DESIGN.md §2；
//! JSON Schema（文档与外部校验器事实源）：schema/profile.schema.json。

pub mod data;
pub mod engines;
pub mod metrics;
pub mod pipeline;
pub mod profile;
pub mod settings;
pub mod store;
pub mod undo;

pub use profile::{
    AnimatedPolicy, CropAnchor, DirKind, ExifPolicy, Format, IccPolicy, JpegEngine, LargerPolicy,
    Metadata, Output, OutputFormat, Pipeline, Profile, Quality, QualityKind, Resize, ResizeMode, Ui,
};
pub use settings::{
    ClipboardSettings, NotifySettings, Settings, SettingsStore, TraySettings, WatchSettings,
    SETTINGS_SCHEMA_VERSION,
};
pub use store::ProfileStore;

/// 测试公用设施：`YALI_HOME` 是进程级环境变量，所有改它的测试必须共用一把锁。
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
}

/// 内核版本号，与 CLI `yali --version` 联动
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_is_semver() {
        let v = CORE_VERSION;
        assert!(v.matches('.').count() >= 2, "not semver: {v}");
    }

    #[test]
    fn embedded_default_fixture_parses_and_validates() {
        let raw = store::FACTORY_PROFILES.iter().find(|(n, _)| *n == "default").unwrap().1;
        let p: Profile = serde_json::from_str(raw).unwrap();
        assert_eq!(p.name, "default");
        assert_eq!(p.quality.level, Some(4));
        assert_eq!(p.metadata.exif, ExifPolicy::Privacy);
        p.validate().unwrap();
    }
}
