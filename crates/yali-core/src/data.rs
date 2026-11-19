//! 全部应用文件共用的数据根；YALI_HOME 仅用于显式重定向。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("YALI_HOME") {
        let path = PathBuf::from(path);
        anyhow::ensure!(path.is_absolute(), "YALI_HOME 必须是绝对路径");
        return Ok(path);
    }
    let home = std::env::var_os("HOME").context("HOME 环境变量未设置，无法定位数据目录")?;
    let home = PathBuf::from(home);
    anyhow::ensure!(home.is_absolute(), "HOME 必须是绝对路径");
    Ok(home.join(".yali"))
}

/// 首次访问复制旧配置；新目录始终优先，旧文件留存，缓存无需迁移。
pub fn import_legacy() -> Result<()> {
    static IMPORT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = IMPORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if std::env::var_os("YALI_HOME").is_some() {
        return Ok(());
    }
    let target = root()?;
    let marker = target.join(".legacy-imported");
    if marker.exists() {
        return Ok(());
    }
    let home = std::env::var_os("HOME").context("HOME 环境变量未设置")?;
    copy_missing(
        &PathBuf::from(home).join("Library/Application Support/app.yali"),
        &target,
    )?;
    std::fs::create_dir_all(&target)?;
    std::fs::write(marker, b"1\n")?;
    Ok(())
}

fn copy_missing(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            copy_missing(&entry.path(), &destination)?;
        } else if kind.is_file() {
            if destination.try_exists()? {
                continue;
            }
            copy_missing_file(std::fs::File::open(entry.path())?, &destination)?;
        }
    }
    Ok(())
}

fn copy_missing_file(mut source: impl std::io::Read, destination: &Path) -> Result<()> {
    // 临时目录与目标同分区；中断只留下未发布的临时文件，重试可重新复制。
    let workspace = TempDir::in_dir(destination.parent().context("迁移目标缺少父目录")?)?;
    let staged = workspace.path().join("content");
    let mut file = std::fs::File::create(&staged)?;
    std::io::copy(&mut source, &mut file)?;
    file.sync_all()?;
    // hard_link 原子发布且不覆盖并发创建的新配置；rename 会覆盖，不能用于此处。
    match std::fs::hard_link(&staged, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

/// 编码中间文件也写入数据根；独立工作目录避免同名输入互相覆盖。
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Result<Self> {
        Self::in_dir(&root()?.join("cache/tmp"))
    }

    fn in_dir(parent: &Path) -> Result<Self> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = parent.join(format!("{}-{stamp}-{next}", std::process::id()));
        std::fs::create_dir_all(parent)?;
        std::fs::create_dir(&dir)?;
        Ok(Self(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_import_does_not_publish_partial_files_and_can_retry() {
        struct Interrupted;
        impl std::io::Read for Interrupted {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("模拟复制中断"))
            }
        }

        let dir = std::env::temp_dir().join(format!("yali-import-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let destination = dir.join("settings.json");
        let partial = std::io::Read::chain(std::io::Cursor::new(b"partial"), Interrupted);
        assert!(copy_missing_file(partial, &destination).is_err());
        assert!(!destination.exists(), "失败复制不得留下最终文件");

        copy_missing_file(std::io::Cursor::new(b"complete"), &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"complete");
        copy_missing_file(std::io::Cursor::new(b"other"), &destination).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"complete", "不能覆盖已有配置");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn legacy_import_preserves_both_old_files_and_new_preferences() {
        let dir = std::env::temp_dir().join(format!("yali-import-{}", std::process::id()));
        let old = dir.join("old");
        let new = dir.join("new");
        std::fs::create_dir_all(old.join("profiles")).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("settings.json"), "old").unwrap();
        std::fs::write(old.join("profiles/custom.json"), "custom").unwrap();
        std::fs::write(new.join("settings.json"), "new").unwrap();
        copy_missing(&old, &new).unwrap();
        assert_eq!(
            std::fs::read_to_string(new.join("settings.json")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(new.join("profiles/custom.json")).unwrap(),
            "custom"
        );
        assert!(old.join("profiles/custom.json").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
