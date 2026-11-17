//! T-002 验收：五档出厂 fixtures 反序列化 + 「读入→再序列化→与原文件逐字段一致」roundtrip。

use yali_core::Profile;

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures");
const NAMES: &[&str] = &["default", "web", "archive", "mail", "screenshot"];

fn fixture_raw(name: &str) -> String {
    std::fs::read_to_string(format!("{FIXTURE_DIR}/{name}.json")).unwrap()
}

#[test]
fn profile_fixtures_roundtrip_field_identical() {
    for name in NAMES {
        let raw = fixture_raw(name);
        let expected: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let profile: Profile = serde_json::from_str(&raw).unwrap();
        assert_eq!(profile.name, *name, "档名应与文件名一致: {name}");
        profile.validate().unwrap_or_else(|e| panic!("fixture '{name}' 语义校验失败: {e}"));
        let actual = serde_json::to_value(&profile).unwrap();
        assert_eq!(actual, expected, "读入→再序列化 与原文件逐字段不一致: {name}");
    }
}

#[test]
fn profile_fixtures_cover_five_factory_profiles() {
    for name in NAMES {
        let raw = fixture_raw(name);
        let profile: Profile = serde_json::from_str(&raw).unwrap();
        match *name {
            "default" => assert_eq!(profile.quality.kind, yali_core::QualityKind::Level),
            "web" => {
                assert_eq!(profile.format.output, yali_core::OutputFormat::Webp);
                assert_eq!(profile.quality.level, Some(5), "webp q75 = 100 - 5×level");
            }
            "archive" => {
                assert_eq!(profile.quality.kind, yali_core::QualityKind::Lossless);
                assert_eq!(profile.format.output, yali_core::OutputFormat::Jxl);
            }
            "mail" => {
                assert_eq!(profile.quality.kind, yali_core::QualityKind::TargetBytes);
                assert_eq!(profile.quality.target_bytes, Some(500_000));
            }
            "screenshot" => {
                assert_eq!(profile.quality.kind, yali_core::QualityKind::Lossless);
                assert_eq!(profile.format.output, yali_core::OutputFormat::Auto);
            }
            _ => unreachable!(),
        }
    }
}
