//! 可再生成文件的容量与过期管理；活动文件/工作区用 OS 文件锁保护。
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub const LIMIT_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub bytes: u64,
    pub protected_bytes: u64,
    pub limit_bytes: u64,
}

pub fn root() -> Result<PathBuf> {
    Ok(crate::data::root()?.join("cache"))
}

pub fn lock(root: &Path) -> Result<File> {
    fs::create_dir_all(root)?;
    anyhow::ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "缓存目录不能是符号链接"
    );
    let path = root.join(".maintenance");
    anyhow::ensure!(!path.is_symlink(), "缓存锁不能是符号链接");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock()?;
    Ok(file)
}

struct Entry {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
    directory: bool,
}

// 不跟随任何符号链接。未知顶层目录不归本管理器所有。
fn entries(root: &Path) -> Result<Vec<Entry>> {
    let mut result = Vec::new();
    for name in ["previews", "tmp", "clipboard"] {
        let dir = root.join(name);
        if dir.is_symlink() || !dir.is_dir() {
            continue;
        }
        for item in fs::read_dir(dir)? {
            let item = item?;
            if item.file_type()?.is_symlink() {
                continue;
            }
            let metadata = match item.metadata() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            let bytes = if metadata.is_dir() {
                directory_bytes(&item.path())?
            } else {
                metadata.len()
            };
            result.push(Entry {
                path: item.path(),
                bytes,
                modified: metadata.modified()?,
                directory: metadata.is_dir(),
            });
        }
    }
    Ok(result)
}

fn directory_bytes(path: &Path) -> Result<u64> {
    let mut total = 0;
    let entries = match fs::read_dir(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_file() {
            match entry.metadata() {
                Ok(value) => total += value.len(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        } else if kind.is_dir() {
            total += directory_bytes(&entry.path())?;
        }
    }
    Ok(total)
}

fn idle(entry: &Entry, now: SystemTime, age: Duration) -> Option<File> {
    let path = if entry.directory {
        entry.path.join(".inuse")
    } else {
        entry.path.clone()
    };
    if path.is_symlink() {
        return None;
    }
    // 老版本/崩溃遗留的无锁目录，只在过期后回收。
    if entry.directory && !path.exists() {
        if now.duration_since(entry.modified).unwrap_or_default() < age {
            return None;
        }
        return OpenOptions::new().create(true).truncate(false).read(true).write(true).open(path).ok();
    }
    let file = OpenOptions::new().read(true).write(true).open(path).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

pub fn status_at(root: &Path) -> Result<Status> {
    let _guard = lock(root)?;
    status_locked(root)
}

fn status_locked(root: &Path) -> Result<Status> {
    let mut status = Status {
        bytes: 0,
        protected_bytes: 0,
        limit_bytes: LIMIT_BYTES,
    };
    for entry in entries(root)? {
        status.bytes += entry.bytes;
        if idle(&entry, SystemTime::now(), MAX_AGE).is_none() {
            status.protected_bytes += entry.bytes;
        }
    }
    Ok(status)
}

pub fn prune_at(root: &Path, clear: bool) -> Result<Status> {
    let _guard = lock(root)?;
    prune_locked(root, clear, LIMIT_BYTES, MAX_AGE, SystemTime::now())
}

/// 调用方持 maintenance 锁，允许发布预览并取得共享租约后原子回收。
pub fn prune_locked(
    root: &Path,
    clear: bool,
    limit: u64,
    age: Duration,
    now: SystemTime,
) -> Result<Status> {
    let mut entries = entries(root)?;
    entries.sort_by_key(|entry| entry.modified);
    let mut bytes: u64 = entries.iter().map(|entry| entry.bytes).sum();
    for entry in entries {
        if !clear && bytes <= limit && now.duration_since(entry.modified).unwrap_or_default() < age
        {
            continue;
        }
        if let Some(lease) = idle(&entry, now, age) {
            drop(lease);
            let result = if entry.directory {
                fs::remove_dir_all(&entry.path)
            } else {
                fs::remove_file(&entry.path)
            };
            if result.is_ok() {
                bytes = bytes.saturating_sub(entry.bytes);
            } else {
                result.with_context(|| format!("无法清理缓存 {}", entry.path.display()))?;
            }
        }
    }
    status_locked(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_expiry_clear_and_active_locks() {
        let root = std::env::temp_dir().join(format!("yali-cache-{}", std::process::id()));
        let previews = root.join("previews");
        fs::create_dir_all(&previews).unwrap();
        let now = SystemTime::now();
        for (name, seconds) in [("old", 100), ("recent", 10), ("active", 200)] {
            let path = previews.join(name);
            fs::write(&path, [1; 8]).unwrap();
            OpenOptions::new().read(true).write(true).open(path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(now - Duration::from_secs(seconds)))
                .unwrap();
        }
        let active = File::open(previews.join("active")).unwrap();
        active.lock_shared().unwrap();
        let _guard = lock(&root).unwrap();
        prune_locked(&root, false, 16, MAX_AGE, now).unwrap();
        assert!(!previews.join("old").exists());
        assert!(previews.join("recent").exists());
        assert!(previews.join("active").exists());
        prune_locked(&root, false, 100, Duration::from_secs(5), now).unwrap();
        assert!(!previews.join("recent").exists());
        assert_eq!(status_locked(&root).unwrap().protected_bytes, 8);
        assert_eq!(prune_locked(&root, true, 0, MAX_AGE, now).unwrap().bytes, 8);
        drop(active);
        assert_eq!(prune_locked(&root, true, 0, MAX_AGE, now).unwrap().bytes, 0);
        fs::write(root.join("untouched"), b"preserved").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("untouched"), previews.join("link")).unwrap();
            std::os::unix::fs::symlink(root.join("missing"), previews.join("broken")).unwrap();
            prune_locked(&root, true, 0, MAX_AGE, now).unwrap();
            assert!(root.join("untouched").exists());
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn another_process_holds_lease_until_it_exits() {
        use std::io::{BufRead, Write};
        use std::process::{Command, Stdio};
        const CHILD_PATH: &str = "YALI_TEST_CACHE_LEASE";
        if let Some(path) = std::env::var_os(CHILD_PATH) {
            let lease = File::open(path).unwrap();
            lease.lock_shared().unwrap();
            println!("lease-ready");
            std::io::stdout().flush().unwrap();
            // 模拟进程异常退出，由父进程终止；锁必须由 OS 自动释放。
            let mut byte = [0];
            std::io::Read::read_exact(&mut std::io::stdin(), &mut byte).unwrap();
            return;
        }
        let root = std::env::temp_dir().join(format!("yali-cache-process-{}", std::process::id()));
        let previews = root.join("previews");
        fs::create_dir_all(&previews).unwrap();
        let path = previews.join("active");
        fs::write(&path, [1; 8]).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cache::tests::another_process_holds_lease_until_it_exits",
                "--nocapture",
            ])
            .env(CHILD_PATH, &path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert!(
                output.read_line(&mut line).unwrap() > 0,
                "子进程未建立缓存租约"
            );
            if line.trim() == "lease-ready" {
                break;
            }
        }
        let held = prune_at(&root, true).unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(held.bytes, 8);
        assert_eq!(held.protected_bytes, 8);
        assert_eq!(prune_at(&root, true).unwrap().bytes, 0);
        fs::remove_dir_all(root).unwrap();
    }
}
