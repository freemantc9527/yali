//! ProfileStore：磁盘档的加载 / 保存 / 列举 / 出厂播种。
//!
//! 数据目录：`~/.yali/profiles/*.json`；
//! 测试与沙盒用 `YALI_HOME` 重定向（`dir()` = `$YALI_HOME/profiles`）。
//! 保存为原子写：先写同目录临时文件，再 rename 覆盖（DESIGN §2 同步机制的前提）。
//! 首次访问自动播种四档出厂 fixtures（`fixtures/*.json`）；出厂档集合新增时
//! 按 `.seed` 签名给老目录补种缺失档，签名一致则不复活用户删掉的档。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::profile::Profile;

/// 出厂档：文件名 → 内嵌 JSON。JSON Schema 见 schema/profile.schema.json。
pub const FACTORY_PROFILES: &[(&str, &str)] = &[
    ("default", include_str!("../../../fixtures/default.json")),
    ("jpg", include_str!("../../../fixtures/jpg.json")),
    (
        "thumbnail",
        include_str!("../../../fixtures/thumbnail.json"),
    ),
    ("archive", include_str!("../../../fixtures/archive.json")),
];

// 旧出厂档仅用于识别升级前的原版档；用户改过的档仍作为自定义预设保留。
const RETIRED_PROFILES: &[(&str, &str)] = &[
    ("web", include_str!("../../../fixtures/web.json")),
    ("mail", include_str!("../../../fixtures/mail.json")),
    (
        "screenshot",
        include_str!("../../../fixtures/screenshot.json"),
    ),
];

#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    /// 解析 profiles 数据目录（尊重 `YALI_HOME` 重定向）
    pub fn dir() -> Result<PathBuf> {
        Ok(crate::data::root()?.join("profiles"))
    }

    /// 默认存储（磁盘数据目录，缺档时播种出厂档）
    pub fn open() -> Result<Self> {
        crate::data::import_legacy()?;
        let dir = Self::dir()?;
        let store = Self { dir };
        store.ensure_seeded()?;
        Ok(store)
    }

    /// 指定目录的存储（测试 / 显式数据根用，不播种）
    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn path_for(&self, name: &str) -> Result<PathBuf> {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            anyhow::bail!("非法 profile 名: '{name}'");
        }
        Ok(self.dir.join(format!("{name}.json")))
    }

    pub fn load(&self, name: &str) -> Result<Profile> {
        let path = self.path_for(name)?;
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("读取 profile '{name}' 失败: {}", path.display()))?;
        let mut profile: Profile = serde_json::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("profile '{name}' 解析失败: {e}"))?;
        profile.validate().map_err(|e| anyhow::anyhow!("profile '{name}' 校验失败: {e}"))?;
        // 在读取时升级完全未改动的旧出厂档，不改写用户文件；回退旧版本仍可读取。
        if let Some((_, raw)) = FACTORY_PROFILES.iter().find(|(factory, _)| *factory == name) {
            let current: Profile = serde_json::from_str(raw)?;
            if current.quality.kind == crate::profile::QualityKind::Recommended {
                let mut previous = current.clone();
                previous.quality.kind = crate::profile::QualityKind::Level;
                if profile == previous { profile = current; }
            }
        }
        Ok(profile)
    }

    /// 原子写：临时文件 + rename（同目录保证同分区，rename 原子）
    pub fn save(&self, profile: &Profile) -> Result<()> {
        profile.validate()?;
        let path = self.path_for(&profile.name)?;
        fs::create_dir_all(&path.parent().unwrap_or(Path::new(".")))?;
        let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        {
            let mut f = fs::File::create(&tmp).with_context(|| format!("写入 {} 失败", tmp.display()))?;
            f.write_all(serde_json::to_string_pretty(profile)?.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &path).with_context(|| format!("原子替换 {} 失败", path.display()))?;
        Ok(())
    }

    /// 磁盘上所有档名（去 .json 后缀，字典序）
    pub fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        if !self.dir.exists() {
            return Ok(names);
        }
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some((_, raw)) = RETIRED_PROFILES.iter().find(|(name, _)| *name == stem)
                    {
                        let factory: Profile = serde_json::from_str(raw)?;
                        if self.load(stem).is_ok_and(|profile| profile == factory) {
                            continue;
                        }
                    }
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// 出厂档签名：档名集合变化（例如新增 jpg 档）就要给老用户补种缺失的档
    fn factory_signature() -> String {
        let mut names: Vec<&str> = FACTORY_PROFILES.iter().map(|(n, _)| *n).collect();
        names.sort();
        format!("{}:{}", names.len(), names.join(","))
    }

    /// 播种出厂档：新装全量播种；老用户若出厂档集合有新增，只补缺失的那几档。
    /// 签名一致时什么都不动——用户主动删掉的出厂档不会被强行复活。
    fn ensure_seeded(&self) -> Result<()> {
        fs::create_dir_all(&self.dir)?;
        self.upgrade_resize_default()?;
        let marker = self.dir.join(".seed");
        let signature = Self::factory_signature();
        if fs::read_to_string(&marker).map(|v| v.trim().to_string()).ok().as_deref() == Some(signature.as_str()) {
            return Ok(());
        }
        for (name, raw) in FACTORY_PROFILES {
            let path = self.dir.join(format!("{name}.json"));
            if !path.exists() {
                fs::write(&path, raw).with_context(|| format!("播种出厂档 '{name}' 失败"))?;
            }
        }
        fs::write(&marker, signature + "\n")?;
        Ok(())
    }

    /// 只升级上一版未修改的 320px 出厂档，自定义参数保持原值。
    fn upgrade_resize_default(&self) -> Result<()> {
        let Some((_, raw)) = FACTORY_PROFILES.iter().find(|(name, _)| *name == "thumbnail") else {
            return Ok(());
        };
        let current: Profile = serde_json::from_str(raw)?;
        let mut previous = current.clone();
        previous.resize.max_width = Some(320);
        previous.resize.max_height = Some(320);
        let mut legacy = previous.clone();
        legacy.quality.kind = crate::profile::QualityKind::Level;
        if self.load("thumbnail").is_ok_and(|profile| profile == previous || profile == legacy) {
            self.save(&current)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 无第三方依赖的临时目录（不引 tempfile，白名单约束）
    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("yali-profile-test-{}-{}-{nanos}", tag, std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_profile(name: &str) -> Profile {
        let raw = FACTORY_PROFILES.iter().find(|(n, _)| *n == "default").unwrap().1;
        let mut p: Profile = serde_json::from_str(raw).unwrap();
        p.name = name.into();
        p
    }

    #[test]
    fn recommended_upgrade_preserves_custom_profiles_and_original_files() {
        let dir = temp_dir("recommended");
        let store = ProfileStore::with_dir(&dir);
        for name in ["default", "jpg", "thumbnail"] {
            let mut previous: Profile = serde_json::from_str(FACTORY_PROFILES.iter()
                .find(|(n, _)| *n == name).unwrap().1).unwrap();
            previous.quality.kind = crate::profile::QualityKind::Level;
            store.save(&previous).unwrap();
            let original = fs::read(store.path_for(name).unwrap()).unwrap();
            assert_eq!(store.load(name).unwrap().quality.kind, crate::profile::QualityKind::Recommended);
            assert_eq!(fs::read(store.path_for(name).unwrap()).unwrap(), original);
            previous.output.suffix_template = "-custom".into();
            store.save(&previous).unwrap();
            assert_eq!(store.load(name).unwrap(), previous);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn profile_store_save_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = ProfileStore::with_dir(&dir);
        let p = sample_profile("default");
        store.save(&p).unwrap();
        let back = store.load("default").unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn profile_store_seeds_new_factory_profiles_for_existing_install() {
        // 老用户：目录里只有五档 + 旧签名 → 补种后必须出现新增的 jpg 档
        let dir = temp_dir("upgrade");
        let profiles = dir.join("profiles");
        fs::create_dir_all(&profiles).unwrap();
        let default_raw = FACTORY_PROFILES.iter().find(|(n, _)| *n == "default").unwrap().1;
        for name in ["archive", "default", "mail", "screenshot", "web"] {
            fs::write(profiles.join(format!("{name}.json")), default_raw).unwrap();
        }
        fs::write(
            profiles.join(".seed"),
            "5:archive,default,mail,screenshot,web\n",
        )
        .unwrap();

        let store = ProfileStore::with_dir(&profiles);
        store.ensure_seeded().unwrap();
        assert!(
            profiles.join("jpg.json").exists(),
            "出厂档集合新增后必须补种"
        );
        assert!(profiles.join("thumbnail.json").exists());
        assert_eq!(store.list().unwrap().len(), 7, ".seed 不应被当成档");
    }

    #[test]
    fn profile_store_does_not_resurrect_deleted_profiles() {
        let dir = temp_dir("noresurrect");
        let store = ProfileStore::with_dir(&dir);
        store.ensure_seeded().unwrap();
        fs::remove_file(dir.join("thumbnail.json")).unwrap();
        store.ensure_seeded().unwrap();
        assert!(
            !dir.join("thumbnail.json").exists(),
            "签名未变时不应复活被删的档"
        );
    }

    #[test]
    fn upgrade_hides_retired_factory_defaults_and_preserves_custom_edits() {
        let dir = temp_dir("retired");
        let store = ProfileStore::with_dir(&dir);
        for (name, raw) in RETIRED_PROFILES {
            fs::write(dir.join(format!("{name}.json")), raw).unwrap();
        }
        store.ensure_seeded().unwrap();
        assert_eq!(
            store.list().unwrap(),
            vec!["archive", "default", "jpg", "thumbnail"]
        );
        assert!(dir.join("web.json").exists(), "旧档原件留存");
        let mut custom = store.load("web").unwrap();
        custom.quality.level = Some(6);
        store.save(&custom).unwrap();
        assert!(store.list().unwrap().contains(&"web".into()));
    }

    #[test]
    fn resize_default_upgrade_updates_320_and_preserves_custom_parameters() {
        let dir = temp_dir("resize-upgrade");
        let store = ProfileStore::with_dir(&dir);
        store.ensure_seeded().unwrap();
        let mut old = store.load("thumbnail").unwrap();
        old.resize.max_width = Some(320);
        old.resize.max_height = Some(320);
        store.save(&old).unwrap();
        store.ensure_seeded().unwrap();
        let upgraded = store.load("thumbnail").unwrap();
        assert_eq!(upgraded.resize.max_width, Some(1024));
        assert_eq!(upgraded.resize.max_height, Some(1024));
        assert!(upgraded.resize.keep_ratio);
        old.quality.level = Some(6);
        store.save(&old).unwrap();
        store.ensure_seeded().unwrap();
        assert_eq!(store.load("thumbnail").unwrap(), old);
    }

    #[test]
    fn profile_store_list_sorted() {
        let dir = temp_dir("list");
        let store = ProfileStore::with_dir(&dir);
        for name in ["web", "archive", "default"] {
            store.save(&sample_profile(name)).unwrap();
        }
        // 混入一个非 .json 文件，不应被列出
        fs::write(dir.join("notes.txt"), "hi").unwrap();
        assert_eq!(store.list().unwrap(), vec!["archive", "default", "web"]);
    }

    #[test]
    fn profile_store_seed_factory_profiles() {
        let dir = temp_dir("seed");
        let store = ProfileStore::open_in(&dir);
        let names = store.list().unwrap();
        assert_eq!(
            names,
            vec!["archive", "default", "jpg", "thumbnail"],
            "首启应播种四档出厂 fixtures"
        );
        for name in names {
            if cfg!(windows) && name == "archive" {
                assert!(store.load(&name).is_err(), "归档保留文件，但 Windows 不执行");
            } else {
                store.load(&name).unwrap();
            }
        }
    }

    #[test]
    fn profile_store_load_reports_line_number_on_bad_json() {
        let dir = temp_dir("badjson");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("broken.json"), "{\n  \"name\": \"broken\",\n  \"oops").unwrap();
        let store = ProfileStore::with_dir(&dir);
        let err = store.load("broken").unwrap_err().to_string();
        assert!(err.contains("line"), "非法 JSON 报错应含行号: {err}");
    }

    #[test]
    fn profile_store_load_reports_unknown_field() {
        let dir = temp_dir("unknown");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bad.json"), r#"{"name":"bad","noSuchField":true}"#).unwrap();
        let store = ProfileStore::with_dir(&dir);
        let err = store.load("bad").unwrap_err().to_string();
        assert!(err.contains("noSuchField"), "未知字段报错应指出字段名: {err}");
    }

    #[test]
    fn profile_store_rejects_path_traversal() {
        let store = ProfileStore::with_dir(temp_dir("traversal"));
        assert!(store.load("../escape").is_err());
        assert!(store.load("a/b").is_err());
        assert!(store.load("").is_err());
    }

    #[test]
    fn profile_store_save_is_atomic_no_tmp_leftover() {
        let dir = temp_dir("atomic");
        let store = ProfileStore::with_dir(&dir);
        store.save(&sample_profile("default")).unwrap();
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "原子写不应残留临时文件: {leftovers:?}");
    }

    #[test]
    fn profile_store_respects_yali_home_env() {
        // env 是进程级全局，串行化避免并发测试互踩（settings 测试共用同一把锁）
        let _guard = crate::test_support::ENV_LOCK.lock().unwrap();
        let root = temp_dir("env");
        std::env::set_var("YALI_HOME", &root);
        let dir = ProfileStore::dir().unwrap();
        std::env::remove_var("YALI_HOME");
        assert_eq!(dir, root.join("profiles"));
    }

    /// 测试辅助：指定目录 + 播种（生产路径 open() 的目录注入版）
    impl ProfileStore {
        fn open_in(dir: &Path) -> Self {
            let store = Self { dir: dir.to_path_buf() };
            store.ensure_seeded().unwrap();
            store
        }
    }
}
