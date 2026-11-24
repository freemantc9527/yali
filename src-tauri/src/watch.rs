//! T-017 watch 文件夹：FSEvents 监视 → 防抖去重 → 入队压缩（输出同目录）。
//!
//! 三个必须防的坑：
//! 1. **自激**：我们的产物落在被监视目录里，会被自己的事件再触发一次 → 产出路径进 ignored 集
//! 2. **抖动**：一个文件写入会触发多次事件 → 按 (path, mtime, size) + settleMs 防抖
//! 3. **重复**：同一 (mtime, size) 只处理一次（重启后重新扫到的旧文件不重压）

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use yali_core::profile::Profile;
use yali_core::Settings;

use crate::supported::is_image_path;

/// 监督循环节拍：150ms 足够接近实时，也不会把 CPU 烧起来
const TICK: Duration = Duration::from_millis(150);

/// 运行期配置（settings_save 会就地替换）
#[derive(Debug, Clone, PartialEq)]
pub struct WatchConfig {
    pub enabled: bool,
    pub dirs: Vec<PathBuf>,
    pub settle: Duration,
}

impl WatchConfig {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            enabled: settings.watch.enabled,
            dirs: settings.watch.dirs.iter().map(PathBuf::from).collect(),
            settle: Duration::from_millis(settings.watch.settle_ms),
        }
    }
}

pub struct WatchHandle {
    config: Arc<Mutex<WatchConfig>>,
}

/// 启动监督线程（线程常驻，enabled=false 时空转，不重建）
pub fn start(app: tauri::AppHandle, settings: &Settings) -> WatchHandle {
    let config = Arc::new(Mutex::new(WatchConfig::from_settings(settings)));
    let handle = WatchHandle { config: config.clone() };
    std::thread::spawn(move || supervise(app, config));
    handle
}

/// 设置变更后就地生效（目录变了会重建 watcher）
pub fn apply(handle: &WatchHandle, settings: &Settings) {
    let mut guard = handle.config.lock().unwrap_or_else(|e| e.into_inner());
    *guard = WatchConfig::from_settings(settings);
}

/// 候选文件：图片 + 不是我们的临时/清单文件
pub fn is_candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else { return false };
    if name.starts_with('.') {
        return false; // .DS_Store / 隐藏文件
    }
    if name.ends_with(".yali-undo.json") || name.contains(".yali-") {
        return false; // 撤销清单 + 管道中转名（{stem}.yali-{pid}.tmp.{ext}）
    }
    is_image_path(path)
}

struct Active {
    dirs: Vec<PathBuf>,
    rx: std::sync::mpsc::Receiver<notify::Result<notify::Event>>,
    _watcher: RecommendedWatcher,
}

impl Active {
    fn new(dirs: &[PathBuf]) -> Option<Self> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            // 接收端没了（监督线程退出）就静默丢弃，不 panic
            let _ = tx.send(event);
        })
        .ok()?;
        for dir in dirs {
            if let Err(e) = watcher.watch(dir, RecursiveMode::Recursive) {
                eprintln!("watch: 无法监视 {}: {e}", dir.display());
            }
        }
        Some(Self { dirs: dirs.to_vec(), rx, _watcher: watcher })
    }
}

/// 防抖状态机（纯逻辑，可单测）
#[derive(Default)]
pub struct Debouncer {
    pending: HashMap<PathBuf, (Instant, u64, u64)>,
    seen: HashMap<PathBuf, (u64, u64)>,
}

impl Debouncer {
    pub fn observe(&mut self, path: PathBuf, mtime: u64, size: u64, now: Instant) {
        self.pending.insert(path, (now, mtime, size));
    }

    /// 超过 settle 窗口的候选；同一 (mtime, size) 只放行一次
    pub fn take_ready(&mut self, now: Instant, settle: Duration) -> Vec<PathBuf> {
        let ready: Vec<PathBuf> = self
            .pending
            .iter()
            .filter(|(_, (last, _, _))| now.duration_since(*last) >= settle)
            .map(|(path, _)| path.clone())
            .collect();

        let mut out = Vec::new();
        for path in ready {
            let Some((_, mtime, size)) = self.pending.remove(&path) else { continue };
            if self.seen.get(&path) == Some(&(mtime, size)) {
                continue; // 处理过同一版本了
            }
            self.seen.insert(path.clone(), (mtime, size));
            out.push(path);
        }
        out
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.seen.clear();
    }
}

fn supervise(
    app: tauri::AppHandle,
    config: Arc<Mutex<WatchConfig>>,
) {
    let ignored: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
    let engines: Arc<tokio::sync::OnceCell<Result<(Profile, Arc<yali_core::engines::registry::Registry>), String>>> =
        Arc::default();
    let mut active: Option<Active> = None;
    let mut debouncer = Debouncer::default();

    loop {
        let cfg = config.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if !cfg.enabled || cfg.dirs.is_empty() {
            if active.take().is_some() {
                debouncer.clear();
            }
            std::thread::sleep(TICK);
            continue;
        }

        let rebuild = active.as_ref().map(|a| a.dirs != cfg.dirs).unwrap_or(true);
        if rebuild {
            active = Active::new(&cfg.dirs);
            debouncer.clear();
        }

        if let Some(current) = active.as_mut() {
            while let Ok(event) = current.rx.try_recv() {
                let Ok(event) = event else { continue };
                for path in event.paths {
                    if !is_candidate(&path) {
                        continue;
                    }
                    if ignored
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .contains(&path)
                    {
                        continue;
                    }
                    let Ok(meta) = std::fs::metadata(&path) else { continue };
                    if !meta.is_file() {
                        continue;
                    }
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0);
                    debouncer.observe(path, mtime, meta.len(), Instant::now());
                }
            }
        }

        for path in debouncer.take_ready(Instant::now(), cfg.settle) {
            spawn_compress(app.clone(), path, ignored.clone(), engines.clone());
        }

        std::thread::sleep(TICK);
    }
}

fn spawn_compress(
    app: tauri::AppHandle,
    path: PathBuf,
    ignored: Arc<Mutex<HashSet<PathBuf>>>,
    engines: Arc<tokio::sync::OnceCell<Result<(Profile, Arc<yali_core::engines::registry::Registry>), String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let ready = engines
            .get_or_init(|| async {
                let profile = yali_core::ProfileStore::open()
                    .and_then(|store| store.load("default"))
                    .map_err(|e| format!("{e:#}"))?;
                let registry = yali_core::engines::registry::Registry::discover(&profile);
                registry.warmup().await;
                Ok::<_, String>((profile, Arc::new(registry)))
            })
            .await;
        let (profile, registry) = match ready {
            Ok(value) => value.clone(),
            Err(e) => {
                eprintln!("watch: 内核初始化失败: {e}");
                return;
            }
        };

        match yali_core::pipeline::compress_file(&path, &profile, &registry).await {
            Ok(report) => {
                if let Some(output) = report.output {
                    // 自己的产物不再触发（自激）
                    ignored
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(PathBuf::from(output));
                }
                let _ = &app; // 预留：后续把结果推进 GUI 任务列表
            }
            Err(e) => eprintln!("watch: 压缩失败 {}: {e:#}", path.display()),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn candidate_filters_hidden_and_pipeline_files() {
        for good in ["photo.jpg", "截图 2026.PNG", "a.gif", "a.webp", "a.bmp", "a.tiff"] {
            assert!(is_candidate(Path::new(good)), "{good} 应被监视");
        }
        assert_eq!(is_candidate(Path::new("a.heic")), !cfg!(windows), "HEIC follows platform input support");
        for bad in [".DS_Store", ".yali-undo.json", "a.yali-123.tmp.png", "notes.txt", "noext"] {
            assert!(!is_candidate(Path::new(bad)), "{bad} 不应被监视");
        }
    }

    #[test]
    fn burst_collapses_to_one_after_settle_window() {
        let mut d = Debouncer::default();
        let t0 = Instant::now();
        let path = PathBuf::from("/tmp/photo.jpg");
        d.observe(path.clone(), 1, 100, t0);
        d.observe(path.clone(), 1, 100, t0 + Duration::from_millis(100));
        d.observe(path.clone(), 1, 100, t0 + Duration::from_millis(200));
        assert!(
            d.take_ready(t0 + Duration::from_millis(300), Duration::from_millis(500)).is_empty(),
            "写入还在继续时不应处理"
        );
        assert_eq!(
            d.take_ready(t0 + Duration::from_millis(800), Duration::from_millis(500)),
            vec![path],
            "窗口结束后只处理一次"
        );
    }

    #[test]
    fn same_mtime_and_size_is_not_reprocessed() {
        let mut d = Debouncer::default();
        let t0 = Instant::now();
        let path = PathBuf::from("/tmp/photo.jpg");
        d.observe(path.clone(), 1, 100, t0);
        assert_eq!(d.take_ready(t0 + Duration::from_secs(1), Duration::ZERO), vec![path.clone()]);
        d.observe(path.clone(), 1, 100, t0 + Duration::from_secs(2));
        assert!(
            d.take_ready(t0 + Duration::from_secs(3), Duration::ZERO).is_empty(),
            "同 (mtime,size) 不应重复压"
        );
        d.observe(path.clone(), 2, 100, t0 + Duration::from_secs(4));
        assert_eq!(
            d.take_ready(t0 + Duration::from_secs(5), Duration::ZERO),
            vec![path],
            "文件变了要重新压"
        );
    }

    #[test]
    fn config_follows_settings() {
        let mut settings = Settings::default();
        settings.watch.enabled = true;
        settings.watch.dirs = vec!["/tmp/yali-watch".into()];
        settings.watch.settle_ms = 1200;
        let cfg = WatchConfig::from_settings(&settings);
        assert!(cfg.enabled);
        assert_eq!(cfg.dirs, vec![PathBuf::from("/tmp/yali-watch")]);
        assert_eq!(cfg.settle, Duration::from_millis(1200));
    }
}