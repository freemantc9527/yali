//! 全部应用文件共用的数据根；YALI_HOME 仅用于显式重定向。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("YALI_HOME") {
        let path = PathBuf::from(path);
        anyhow::ensure!(path.is_absolute(), "YALI_HOME 必须是绝对路径");
        return Ok(path);
    }
    Ok(home()?.join(".yali"))
}

pub fn home() -> Result<PathBuf> {
    #[cfg(windows)]
    let path = std::env::var_os("USERPROFILE").filter(|p| !p.is_empty()).or_else(|| std::env::var_os("HOME"));
    #[cfg(not(windows))]
    let path = std::env::var_os("HOME");
    let path = PathBuf::from(path.context("无法定位用户主目录（Windows 使用 USERPROFILE，其他平台使用 HOME）")?);
    anyhow::ensure!(path.is_absolute(), "用户主目录必须是绝对路径");
    Ok(path)
}

pub fn downloads() -> Result<PathBuf> {
    dirs::download_dir().context("无法定位系统下载目录，请选择自定义输出目录")
}

/// 首次访问复制旧配置；新目录始终优先，旧文件留存，缓存无需迁移。
pub fn import_legacy() -> Result<()> {
    static IMPORT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = IMPORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if std::env::var_os("YALI_HOME").is_some() {
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    return Ok(());
    #[cfg(target_os = "macos")]
    {
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
    drop(file);
    // hard_link 原子发布且不覆盖并发创建的新配置；rename 会覆盖，不能用于此处。
    match std::fs::hard_link(&staged, destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

/// 编码中间文件也写入数据根；独立工作目录避免同名输入互相覆盖。
pub struct TempDir(PathBuf, Option<std::fs::File>);

impl TempDir {
    pub fn new() -> Result<Self> {
        let cache = crate::cache::root()?;
        crate::cache::prune_at(&cache, false)?;
        let _guard = crate::cache::lock(&cache)?;
        Self::in_dir(&cache.join("tmp"))
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
        let lease = std::fs::File::create(dir.join(".inuse"))?;
        lease.lock()?;
        Ok(Self(dir, Some(lease)))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(self.1.take());
        let cache = self.0.parent().filter(|p| p.file_name().is_some_and(|n| n == "tmp"))
            .and_then(|p| p.parent()).filter(|p| p.file_name().is_some_and(|n| n == "cache"));
        let _guard = cache.and_then(|root| crate::cache::lock(root).ok());
        let _ = std::fs::remove_dir_all(&self.0);
        if let Some(root) = cache.filter(|_| _guard.is_some()) {
            let _ = crate::cache::prune_locked(root, false, crate::cache::LIMIT_BYTES, crate::cache::MAX_AGE, std::time::SystemTime::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn windows_root_works_without_home_and_keeps_override() {
        let _lock = crate::test_support::ENV_LOCK.lock().unwrap();
        let keys = ["HOME", "USERPROFILE", "YALI_HOME"];
        let previous = keys.map(|key| std::env::var_os(key));
        let user = std::env::temp_dir().join("普通 用户");
        std::env::remove_var("HOME");
        std::env::remove_var("YALI_HOME");
        std::env::set_var("USERPROFILE", &user);
        assert_eq!(home().unwrap(), user);
        assert_eq!(root().unwrap(), user.join(".yali"));
        let isolated = user.join("隔离");
        std::env::set_var("YALI_HOME", &isolated);
        assert_eq!(root().unwrap(), isolated);
        for (key, value) in keys.into_iter().zip(previous) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn temporary_workspaces_are_protected_and_crash_leftovers_expire() {
        let root = std::env::temp_dir().join(format!("yali-workspace-{}", std::process::id()));
        let cache = root.join("cache");
        let workspace = TempDir::in_dir(&cache.join("tmp")).unwrap();
        let path = workspace.path().to_path_buf();
        std::fs::write(path.join("content"), [1; 8]).unwrap();
        assert_eq!(crate::cache::prune_at(&cache, true).unwrap().protected_bytes, 8);
        assert!(path.join("content").exists());
        drop(workspace);
        assert!(!path.exists(), "正常结束立即删除工作区");

        let abandoned = cache.join("tmp/legacy-abandoned");
        std::fs::create_dir_all(&abandoned).unwrap();
        std::fs::write(abandoned.join("content"), [1; 8]).unwrap();
        crate::cache::prune_at(&cache, true).unwrap();
        assert!(abandoned.exists(), "没有租约的旧目录在过期前保持安全");
        let expired = std::time::SystemTime::now() - crate::cache::MAX_AGE - std::time::Duration::from_secs(1);
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.write(true).custom_flags(0x02000000); // FILE_FLAG_BACKUP_SEMANTICS
        }
        options.open(&abandoned).unwrap().set_times(std::fs::FileTimes::new().set_modified(expired)).unwrap();
        assert_eq!(crate::cache::prune_at(&cache, false).unwrap().bytes, 0);
        assert!(!abandoned.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

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
