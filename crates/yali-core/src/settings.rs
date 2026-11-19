//! 应用级设置：`settings.json`（不是压图参数，压图参数在 Profile）。
//!
//! 数据位置：`~/.yali/settings.json`；
//! 测试与沙盒用 `YALI_HOME` 重定向（与 ProfileStore 同源）。
//!
//! 三种异常的处理刻意分开（T-014）：
//! - 文件不存在 → 内存默认值（入口默认全关，不偷偷监听）
//! - JSON 语法坏了 → 备份为 `settings.json.bad-<毫秒>`，回落默认值，绝不崩
//! - 字段拼错/类型不对（serde `Category::Data`）→ **报错**，不静默吃配置
//! 版本高于当前（例如未来 v2）→ 报错，绝不覆盖用户更新的配置。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// 当前 settings schema 版本（字段增删时必须 +1 并写迁移）
pub const SETTINGS_SCHEMA_VERSION: u32 = 2;
pub const FONT_SCALE_STEPS: [u16; 6] = [80, 100, 120, 150, 180, 200];

fn default_schema_version() -> u32 {
    SETTINGS_SCHEMA_VERSION
}

fn default_min_bytes() -> u64 {
    64 * 1024
}

fn default_settle_ms() -> u64 {
    800
}

fn default_true() -> bool {
    true
}

fn default_font_scale() -> u16 {
    100
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_font_scale")]
    pub font_scale_percent: u16,
    #[serde(default)]
    pub clipboard: ClipboardSettings,
    #[serde(default)]
    pub watch: WatchSettings,
    #[serde(default)]
    pub tray: TraySettings,
    #[serde(default)]
    pub notify: NotifySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            font_scale_percent: default_font_scale(),
            clipboard: ClipboardSettings::default(),
            watch: WatchSettings::default(),
            tray: TraySettings::default(),
            notify: NotifySettings::default(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version == 0 {
            anyhow::bail!("settings.schemaVersion 不能为 0");
        }
        if self.schema_version > SETTINGS_SCHEMA_VERSION {
            anyhow::bail!(
                "settings.schemaVersion={} 比当前支持的 v{} 新，请升级 yali（不覆盖该文件）",
                self.schema_version,
                SETTINGS_SCHEMA_VERSION
            );
        }
        if self.clipboard.min_bytes > 1 << 30 {
            anyhow::bail!("settings.clipboard.minBytes 过大（上限 1GiB）");
        }
        if !FONT_SCALE_STEPS.contains(&self.font_scale_percent) {
            anyhow::bail!("settings.fontScalePercent 必须为 80 / 100 / 120 / 150 / 180 / 200");
        }
        if self.watch.settle_ms > 10_000 {
            anyhow::bail!("settings.watch.settleMs 上限 10000ms");
        }
        let mut seen = std::collections::HashSet::new();
        for dir in &self.watch.dirs {
            let path = Path::new(dir);
            if !path.is_absolute() {
                anyhow::bail!("settings.watch.dirs 必须是绝对路径: '{dir}'");
            }
            if !seen.insert(dir.clone()) {
                anyhow::bail!("settings.watch.dirs 有重复项: '{dir}'");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClipboardSettings {
    #[serde(default)]
    pub enabled: bool,
    /// 小于该体积的剪贴板图片跳过（表情、截图碎片不值得压）
    #[serde(default = "default_min_bytes")]
    pub min_bytes: u64,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        Self { enabled: false, min_bytes: default_min_bytes() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub dirs: Vec<String>,
    /// 防抖窗口：FSEvents 会在文件写入过程中连续触发
    #[serde(default = "default_settle_ms")]
    pub settle_ms: u64,
}

impl Default for WatchSettings {
    fn default() -> Self {
        Self { enabled: false, dirs: Vec::new(), settle_ms: default_settle_ms() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for TraySettings {
    fn default() -> Self {
        Self { enabled: default_true() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotifySettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for NotifySettings {
    fn default() -> Self {
        Self { enabled: default_true() }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// `$YALI_HOME/settings.json` 或 `~/.yali/settings.json`
    pub fn path() -> Result<PathBuf> {
        Ok(crate::data::root()?.join("settings.json"))
    }

    /// 默认位置（父目录不存在会建）
    pub fn open() -> Result<Self> {
        crate::data::import_legacy()?;
        let path = Self::path()?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .with_context(|| format!("创建配置目录失败: {}", dir.display()))?;
        }
        Ok(Self { path })
    }

    /// 指定文件路径（测试 / 沙盒用）
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn file(&self) -> &Path {
        &self.path
    }

    /// 读取配置：缺失→默认；语法损坏→备份+默认；语义错误（字段拼错/版本过新）→ 报错
    pub fn load(&self) -> Result<Settings> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(e).with_context(|| format!("读取 {} 失败", self.path.display())),
        };

        match serde_json::from_str::<Settings>(&raw) {
            Ok(mut settings) => {
                // 旧版连续字号归入最近固定档；越界值和未来版本仍按原规则报错。
                if settings.schema_version <= SETTINGS_SCHEMA_VERSION
                    && (80..=200).contains(&settings.font_scale_percent)
                {
                    settings.font_scale_percent = *FONT_SCALE_STEPS.iter()
                        .min_by_key(|percent| percent.abs_diff(settings.font_scale_percent))
                        .unwrap();
                }
                settings.validate()?;
                // v1 → v2：新增字号默认为 100%，只在下一次保存时写回。
                settings.schema_version = SETTINGS_SCHEMA_VERSION;
                Ok(settings)
            }
            Err(e) if e.classify() == serde_json::error::Category::Data => {
                // 字段拼错 / 类型不对：必须让人看见，不能悄悄回落默认
                Err(anyhow::anyhow!("{} 内容有误: {e}", self.path.display()))
            }
            Err(e) => {
                // 语法坏 / 截断：备份后回落默认，GUI 不因此起不来
                let backup = self.quarantine()?;
                eprintln!(
                    "⚠️ {} 解析失败（{e}），已备份到 {}，本次使用默认配置",
                    self.path.display(),
                    backup.display()
                );
                Ok(Settings::default())
            }
        }
    }

    /// 原子写：临时文件 + rename（同目录保证同分区）
    pub fn save(&self, settings: &Settings) -> Result<()> {
        settings.validate()?;
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp = self.path.with_extension(format!("json.tmp-{}-{now}", std::process::id()));
        {
            let mut f = fs::File::create(&tmp)
                .with_context(|| format!("写入 {} 失败", tmp.display()))?;
            f.write_all(serde_json::to_string_pretty(settings)?.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("原子替换 {} 失败", self.path.display()))?;
        Ok(())
    }

    /// 恢复默认：旧文件先备份（`.bak-<毫秒>`），再写回默认值
    pub fn reset(&self) -> Result<Settings> {
        if self.path.exists() {
            let backup = self.backup_path("bak")?;
            fs::rename(&self.path, &backup)
                .with_context(|| format!("备份 {} 失败", self.path.display()))?;
        }
        let settings = Settings::default();
        self.save(&settings)?;
        Ok(settings)
    }

    fn backup_path(&self, tag: &str) -> Result<PathBuf> {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        Ok(self.path.with_extension(format!("json.{tag}-{millis}")))
    }

    /// 损坏文件改名留档（不删除用户数据）
    fn quarantine(&self) -> Result<PathBuf> {
        let backup = self.backup_path("bad")?;
        fs::rename(&self.path, &backup)
            .with_context(|| format!("备份损坏配置 {} 失败", self.path.display()))?;
        Ok(backup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ENV_LOCK;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "yali-settings-test-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn store(tag: &str) -> (PathBuf, SettingsStore) {
        let dir = temp_dir(tag);
        let store = SettingsStore::with_path(dir.join("settings.json"));
        (dir, store)
    }

    #[test]
    fn defaults_are_valid_and_all_entrypoints_off() {
        let s = Settings::default();
        s.validate().unwrap();
        assert!(!s.clipboard.enabled, "剪贴板监听默认必须关闭");
        assert!(!s.watch.enabled, "watch 默认必须关闭");
        assert_eq!(s.clipboard.min_bytes, 64 * 1024);
        assert_eq!(s.watch.settle_ms, 800);
        assert_eq!(s.schema_version, SETTINGS_SCHEMA_VERSION);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let (_dir, store) = store("missing");
        assert_eq!(store.load().unwrap(), Settings::default());
    }

    #[test]
    fn save_load_roundtrip() {
        let (_dir, store) = store("roundtrip");
        let mut s = Settings::default();
        s.clipboard.enabled = true;
        s.watch.enabled = true;
        s.watch.dirs = vec!["/Users/x/Desktop".into()];
        store.save(&s).unwrap();
        assert_eq!(store.load().unwrap(), s);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let (dir, store) = store("partial");
        fs::write(
            dir.join("settings.json"),
            r#"{"clipboard":{"enabled":true}}"#,
        )
        .unwrap();
        let s = store.load().unwrap();
        assert!(s.clipboard.enabled);
        assert_eq!(s.clipboard.min_bytes, 64 * 1024);
        assert_eq!(s.watch, WatchSettings::default());
        assert!(s.notify.enabled);
    }

    #[test]
    fn v1_settings_migrate_and_font_scale_survives_restart() {
        let (dir, store) = store("font-scale");
        fs::write(
            dir.join("settings.json"),
            r#"{"schemaVersion":1,"tray":{"enabled":false}}"#,
        )
        .unwrap();
        let mut settings = store.load().unwrap();
        assert_eq!(settings.schema_version, 2);
        assert_eq!(settings.font_scale_percent, 100);
        assert!(!settings.tray.enabled);
        settings.font_scale_percent = 180;
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap().font_scale_percent, 180);
        for percent in [0, 79, 81, 110, 175, 201] {
            settings.font_scale_percent = percent;
            assert!(store.save(&settings).is_err());
        }
        assert_eq!(store.load().unwrap().font_scale_percent, 180);
    }

    #[test]
    fn continuous_legacy_font_scales_snap_to_nearest_step() {
        let (dir, store) = store("font-steps");
        for percent in FONT_SCALE_STEPS {
            let mut settings = Settings::default();
            settings.font_scale_percent = percent;
            store.save(&settings).unwrap();
            assert_eq!(store.load().unwrap(), settings);
        }
        fs::write(dir.join("settings.json"), r#"{"schemaVersion":2,"fontScalePercent":175,"tray":{"enabled":false}}"#).unwrap();
        let settings = store.load().unwrap();
        assert_eq!(settings.font_scale_percent, 180);
        assert!(!settings.tray.enabled);
    }

    #[test]
    fn unknown_field_is_rejected_not_silently_ignored() {
        let (dir, store) = store("unknown");
        fs::write(
            dir.join("settings.json"),
            r#"{"clipbord":{"enabled":true}}"#,
        )
        .unwrap();
        let err = store.load().unwrap_err().to_string();
        assert!(err.contains("clipbord"), "拼错字段必须报出来: {err}");
        assert!(
            dir.join("settings.json").exists(),
            "语义错误不应隔离/删除用户配置"
        );
    }

    #[test]
    fn broken_json_is_quarantined_and_defaults_used() {
        let (dir, store) = store("broken");
        fs::write(dir.join("settings.json"), "{\"watch\": {\"enabled\": tr").unwrap();
        let s = store.load().unwrap();
        assert_eq!(s, Settings::default(), "损坏配置应回落默认");
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".bad-"))
            .collect();
        assert_eq!(backups.len(), 1, "损坏文件必须留档: {backups:?}");
        assert!(!dir.join("settings.json").exists());
    }

    #[test]
    fn future_schema_version_is_rejected() {
        let (dir, store) = store("future");
        fs::write(dir.join("settings.json"), r#"{"schemaVersion":3}"#).unwrap();
        let err = store.load().unwrap_err().to_string();
        assert!(err.contains("schemaVersion=3"), "版本过新必须报错: {err}");
        assert!(dir.join("settings.json").exists(), "不能覆盖未来的配置");
    }

    #[test]
    fn relative_or_duplicate_watch_dirs_are_rejected() {
        let mut s = Settings::default();
        s.watch.dirs = vec!["Desktop".into()];
        assert!(s.validate().is_err(), "相对路径必须拒");

        let mut s = Settings::default();
        s.watch.dirs = vec!["/tmp/a".into(), "/tmp/a".into()];
        assert!(s.validate().is_err(), "重复目录必须拒");
    }

    #[test]
    fn save_is_atomic_without_tmp_leftover() {
        let (dir, store) = store("atomic");
        store.save(&Settings::default()).unwrap();
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "原子写不应残留临时文件: {leftovers:?}");
    }

    #[test]
    fn reset_backs_up_old_file_and_writes_defaults() {
        let (dir, store) = store("reset");
        let mut s = Settings::default();
        s.clipboard.enabled = true;
        store.save(&s).unwrap();
        let back = store.reset().unwrap();
        assert_eq!(back, Settings::default());
        assert_eq!(store.load().unwrap(), Settings::default());
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(".bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "reset 应备份旧配置: {backups:?}");
    }

    #[test]
    fn settings_store_respects_yali_home_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_dir("env");
        std::env::set_var("YALI_HOME", &root);
        let path = SettingsStore::path().unwrap();
        std::env::remove_var("YALI_HOME");
        assert_eq!(path, root.join("settings.json"));
    }
}
