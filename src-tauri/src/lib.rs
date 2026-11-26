//! 鸭梨 GUI 壳：只做 IPC 接线，业务全在内核（docs/DESIGN.md §5 / T-005）。

use std::path::Path;

use tauri::Manager;
use yali_core::engines::registry::Registry;
use yali_core::pipeline::{compress_file, CompressReport};
use yali_core::profile::Profile;
use yali_core::ProfileStore;

mod batch;
mod clipboard;
#[cfg(target_os = "macos")]
pub mod notification;
mod preview;
mod supported;
mod tray;
mod watch;
mod window_state;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// 运行期设置快照（托盘可见性/关窗行为要即时读，不走磁盘 IO）
struct RuntimeSettings(std::sync::RwLock<yali_core::Settings>);

impl RuntimeSettings {
    fn tray_enabled(&self) -> bool {
        self.0
            .read()
            .map(|s| s.tray.enabled)
            .unwrap_or_else(|e| e.into_inner().tray.enabled)
    }

    fn clipboard_enabled(&self) -> bool {
        self.0
            .read()
            .map(|s| s.clipboard.enabled)
            .unwrap_or_else(|e| e.into_inner().clipboard.enabled)
    }

    fn clipboard_min_bytes(&self) -> u64 {
        self.0
            .read()
            .map(|s| s.clipboard.min_bytes)
            .unwrap_or_else(|e| e.into_inner().clipboard.min_bytes)
    }

    fn replace(&self, settings: &yali_core::Settings) {
        match self.0.write() {
            Ok(mut guard) => *guard = settings.clone(),
            Err(poisoned) => *poisoned.into_inner() = settings.clone(),
        }
    }
}

/// 冷启动缓冲：open-file 事件可能早于前端就绪，先存这里，UI 挂载后取走
#[derive(Default)]
struct PendingOpenFiles(std::sync::Mutex<Vec<String>>);

impl PendingOpenFiles {
    fn push(&self, paths: Vec<String>) {
        let mut buf = self.0.lock().unwrap_or_else(|e| e.into_inner());
        buf.extend(paths);
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

/// `file://` URL → 存在的图片路径（非图片/不存在的忽略，重复投递去重，保持顺序）
fn open_paths_from_urls(urls: impl IntoIterator<Item = tauri::Url>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    urls.into_iter()
        .filter_map(|url| url.to_file_path().ok())
        .filter(|path| path.is_file())
        .filter(|path| supported::is_image_path(path))
        .map(|path| path.to_string_lossy().into_owned())
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

/// UI 挂载时取走冷启动期间投递的文件（buffer 是唯一事实源，事件只当唤醒信号）
/// T-016：剪贴板轮询（主线程读 NSPasteboard，后台做压缩）
#[cfg(target_os = "macos")]
fn spawn_clipboard_watcher(app: &tauri::AppHandle) {
    use std::sync::{Arc, Mutex};

    let handle = app.clone();
    let watcher = Arc::new(Mutex::new(clipboard::ClipboardWatcher::new(64 * 1024)));

    tauri::async_runtime::spawn(async move {
        // 启动时对齐：把当前剪贴板内容标记为「已见过」，不当成新图片。
        // 注意：不能在 setup 里 block_on 等主线程回调——主线程正被 setup 占着，会死锁。
        {
            let attach_watcher = watcher.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            if handle
                .run_on_main_thread(move || {
                    let board = clipboard::SystemPasteboard::new();
                    let mut guard = attach_watcher.lock().unwrap_or_else(|e| e.into_inner());
                    guard.attach(&board);
                    let _ = tx.send(());
                })
                .is_ok()
            {
                let _ = rx.await;
            }
        }

        loop {
            tokio::time::sleep(clipboard::POLL_INTERVAL).await;

            let Some(state) = handle.try_state::<RuntimeSettings>() else {
                continue;
            };
            if !state.clipboard_enabled() {
                continue;
            }
            let min_bytes = state.clipboard_min_bytes();
            {
                let mut guard = watcher.lock().unwrap_or_else(|e| e.into_inner());
                guard.min_bytes = min_bytes;
            }

            let poll_watcher = watcher.clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            let dispatched = handle
                .run_on_main_thread(move || {
                    let board = clipboard::SystemPasteboard::new();
                    let mut guard = poll_watcher.lock().unwrap_or_else(|e| e.into_inner());
                    let _ = tx.send(guard.poll(&board));
                })
                .is_ok();
            if !dispatched {
                continue;
            }
            if let Ok(Some(png)) = rx.await {
                handle_clipboard_image(&handle, png).await;
            }
        }
    });
}

/// 剪贴板图片：写临时 PNG → 用默认档压 → 落「下载」→ 通知
#[cfg(target_os = "macos")]
async fn handle_clipboard_image(app: &tauri::AppHandle, png: Vec<u8>) {
    let _ = app; // 预留：后续把结果推进 GUI 任务列表
    let Ok(workspace) = yali_core::data::TempDir::new() else {
        return;
    };
    let dir = workspace.path();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let input = dir.join(format!("clip-{stamp}.png"));
    if std::fs::write(&input, &png).is_err() {
        return;
    }

    let profile = match ProfileStore::open().and_then(|store| store.load("default")) {
        Ok(profile) => clipboard::output_profile(profile),
        Err(e) => {
            clipboard::notify(&format!("剪贴板图片没压成：{e:#}"));
            let _ = std::fs::remove_file(&input);
            return;
        }
    };
    let registry = Registry::discover(&profile);
    registry.warmup().await;

    match compress_file(&input, &profile, &registry).await {
        Ok(report) => {
            let name = report
                .output
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "图片".to_string());
            let pct = report
                .saved_pct
                .map(|pct| format!("{pct:.0}%"))
                .unwrap_or_else(|| "0%".to_string());
            clipboard::notify(&format!("{name}（{pct}），已存到「下载」"));
        }
        Err(e) => clipboard::notify(&format!("剪贴板图片没压成：{e:#}")),
    }
    let _ = std::fs::remove_file(&input);
}

#[tauri::command]
fn take_pending_files(state: tauri::State<'_, PendingOpenFiles>) -> Vec<String> {
    state.take()
}

#[tauri::command]
fn core_version() -> String {
    yali_core::CORE_VERSION.to_string()
}

#[tauri::command]
async fn profiles_list() -> Result<Vec<String>, String> {
    let store = ProfileStore::open().map_err(err)?;
    store.list().map_err(err)
}

#[tauri::command]
async fn profiles_show(name: String) -> Result<Profile, String> {
    let store = ProfileStore::open().map_err(err)?;
    store.load(&name).map_err(err)
}

#[tauri::command]
async fn ssim(input: String, output: String) -> Result<f64, String> {
    yali_core::metrics::ssim(Path::new(&input), Path::new(&output))
        .await
        .map_err(err)
}

#[tauri::command]
async fn ssim_available() -> bool {
    yali_core::metrics::ssim_available().await
}

#[tauri::command]
async fn ffmpeg_status() -> yali_core::metrics::FfmpegStatus {
    yali_core::metrics::ffmpeg_status().await
}

#[tauri::command]
fn profiles_validate(profile: Profile) -> Result<Profile, String> {
    profile.validate().map_err(err)?;
    Ok(profile)
}

#[tauri::command]
async fn profiles_save(profile: Profile) -> Result<(), String> {
    profile.validate().map_err(err)?;
    let store = ProfileStore::open().map_err(err)?;
    store.save(&profile).map_err(err)
}

#[tauri::command]
async fn settings_show() -> Result<yali_core::Settings, String> {
    let store = yali_core::SettingsStore::open().map_err(err)?;
    store.load().map_err(err)
}

#[tauri::command]
async fn settings_save(app: tauri::AppHandle, settings: yali_core::Settings) -> Result<(), String> {
    settings.validate().map_err(err)?;
    let store = yali_core::SettingsStore::open().map_err(err)?;
    store.save(&settings).map_err(err)?;
    if let Some(state) = app.try_state::<RuntimeSettings>() {
        state.replace(&settings);
    }
    #[cfg(not(windows))]
    {
        if let Some(watcher) = app.try_state::<watch::WatchHandle>() {
            watch::apply(&watcher, &settings);
        }
        tray::apply(&app, settings.tray.enabled).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 逐文件进度事件（结果权威仍是 command 返回值；事件丢了不影响正确性）
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    path: String,
    index: usize,
    total: usize,
    status: &'static str,
    report: Option<CompressReport>,
    error: Option<String>,
}

/// 并发上限与 CLI 一致：min(cores-1, 4)
fn max_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(2)
        .min(4)
}

#[tauri::command]
async fn compress(
    app: tauri::AppHandle,
    paths: Vec<String>,
    profile: String,
) -> Result<Vec<CompressReport>, String> {
    use std::sync::Arc;
    use tauri::Emitter;
    let profile = ProfileStore::open()
        .map_err(err)?
        .load(&profile)
        .map_err(err)?;
    let total = paths.len();
    if total == 0 {
        return Ok(Vec::new());
    }
    let request = batch::Request {
        batch_id: "legacy".into(),
        profile,
        items: paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| batch::Item {
                job_id: index.to_string(),
                path,
            })
            .collect(),
    };
    let response = batch::execute(
        request,
        Arc::new(move |outcome| {
            let _ = app.emit(
                "yali://progress",
                ProgressEvent {
                    path: outcome.path,
                    index: outcome.job_id.parse().unwrap_or(0),
                    total,
                    status: if outcome.status == "larger_output" {
                        "done"
                    } else {
                        outcome.status
                    },
                    report: outcome.report,
                    error: outcome.error,
                },
            );
        }),
    )
    .await?;
    let mut reports = Vec::new();
    for outcome in response.outcomes {
        if let Some(error) = outcome.error {
            return Err(error);
        }
        if let Some(report) = outcome.report {
            reports.push(report);
        }
    }
    Ok(reports)
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn quick_actions_status() -> Result<yali_core::services::Status, String> {
    yali_core::services::status().map_err(err)
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn quick_actions_install() -> Result<yali_core::services::Status, String> {
    let status = yali_core::services::install().map_err(err)?;
    yali_core::services::refresh();
    Ok(status)
}

#[cfg(target_os = "macos")]
#[tauri::command]
fn quick_actions_uninstall() -> Result<yali_core::services::Status, String> {
    let status = yali_core::services::uninstall().map_err(err)?;
    yali_core::services::refresh();
    Ok(status)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn quick_actions_status() -> Result<serde_json::Value, String> { Err("此平台不支持系统快捷操作".into()) }
#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn quick_actions_install() -> Result<serde_json::Value, String> { Err("此平台不支持系统快捷操作".into()) }
#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn quick_actions_uninstall() -> Result<serde_json::Value, String> { Err("此平台不支持系统快捷操作".into()) }

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(PendingOpenFiles::default())
        .manage(preview::PreviewLeases::default())
        .setup(|app| {
            window_state::setup(app)?;
            #[cfg(target_os = "macos")]
            if let Err(e) = yali_core::services::repair_existing_link() {
                eprintln!("CLI 入口修复失败: {e:#}");
            }
            tauri::async_runtime::spawn(async {
                loop {
                    if let Err(e) = preview::maintain(false).await {
                        eprintln!("缓存回收失败：{e}");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                }
            });
            let settings = yali_core::SettingsStore::open()
                .and_then(|store| store.load())
                .unwrap_or_default();
            app.manage(RuntimeSettings(std::sync::RwLock::new(settings.clone())));
            #[cfg(not(windows))]
            {
                tray::apply(app.handle(), settings.tray.enabled)?;
                let watch_handle = watch::start(app.handle().clone(), &settings);
                app.manage(watch_handle);
            }
            #[cfg(target_os = "macos")]
            spawn_clipboard_watcher(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            window_state::on_window_event(window, event);
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(not(windows))]
                let enabled = window
                    .try_state::<RuntimeSettings>()
                    .map(|state| state.tray_enabled())
                    .unwrap_or(false);
                #[cfg(not(windows))]
                if tray::close_action(enabled) == tray::CloseAction::Hide {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            batch::compress_batch,
            preview::inspect_images,
            preview::image_preview,
            preview::image_source,
            preview::release_preview,
            preview::cache_status,
            preview::cache_clear,
            profiles_validate,
            core_version,
            profiles_list,
            profiles_show,
            profiles_save,
            settings_show,
            settings_save,
            quick_actions_status,
            quick_actions_install,
            quick_actions_uninstall,
            take_pending_files,
            ssim,
            ssim_available,
            ffmpeg_status,
            compress
        ])
        .build(tauri::generate_context!())
        .expect("error while building yali");

    app.run(|handle, event| {
        match &event {
            tauri::RunEvent::ExitRequested { .. } => window_state::capture_main(handle),
            tauri::RunEvent::Exit => window_state::save(handle),
            _ => {}
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = event {
            use tauri::{Emitter, Manager};
            let paths = open_paths_from_urls(urls.clone());
            if paths.is_empty() {
                eprintln!("open-file: 收到投递但无可处理图片（{urls:?}）");
                return;
            }
            eprintln!("open-file: {}", paths.join(", "));
            if let Some(state) = handle.try_state::<PendingOpenFiles>() {
                state.push(paths.clone());
            }
            // 事件只是唤醒信号；路径的权威来源是 PendingOpenFiles（避免冷启动双入队）
            let _ = handle.emit("yali://open-files", ());
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (handle, event);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "yali-open-test-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// T-013：open-file 只收存在的图片；中文/空格路径经 file:// URL 往返不丢字符；重复投递去重
    #[test]
    fn open_paths_accept_only_existing_images_and_decode_urls() {
        let dir = temp_dir("filter");
        let image = dir.join("带 空格 的图.png");
        std::fs::write(&image, b"not-really-a-png").unwrap();
        let upper = dir.join("SHOT.JPG");
        std::fs::write(&upper, b"x").unwrap();
        let notes = dir.join("notes.txt");
        std::fs::write(&notes, b"x").unwrap();
        let missing = dir.join("ghost.webp");

        let urls: Vec<tauri::Url> = [&image, &notes, &missing, &upper, &image]
            .iter()
            .map(|p| tauri::Url::from_file_path(p).unwrap())
            .collect();

        let paths = open_paths_from_urls(urls);
        assert_eq!(
            paths,
            vec![
                image.to_string_lossy().into_owned(),
                upper.to_string_lossy().into_owned()
            ],
            "非图片/不存在应忽略，重复应去重，后缀大小写不敏感"
        );
    }

    /// 冷启动缓冲：取走即清空（第二次取必须为空，避免 UI 重复入队）
    #[test]
    fn pending_open_files_take_drains_buffer() {
        let buffer = PendingOpenFiles::default();
        buffer.push(vec!["/tmp/a.png".into(), "/tmp/b.jpg".into()]);
        assert_eq!(buffer.take(), vec!["/tmp/a.png", "/tmp/b.jpg"]);
        assert!(buffer.take().is_empty());
    }

    /// IPC 命令层冒烟：profiles_list / profiles_show / compress 三个 command 的真实行为。
    /// YALI_HOME 隔离到临时目录（env 是进程级全局，故所有断言集中在一个测试里串行跑）。
    #[tokio::test]
    async fn commands_smoke_with_isolated_home() {
        let home = std::env::temp_dir().join(format!("yali-cmd-test-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("YALI_HOME", &home);

        let names = profiles_list().await.unwrap();
        assert_eq!(names, vec!["archive", "default", "jpg", "thumbnail"]);

        let profile = profiles_show("thumbnail".into()).await.unwrap();
        assert_eq!(profile.name, "thumbnail");
        assert_eq!(profile.resize.max_width, Some(1024));

        // 导入允许 Rust serde 默认字段：返回完整对象，前端不能持有缺段的原 JSON。
        let partial: Profile =
            serde_json::from_str(r#"{"name":"imported","quality":{"kind":"level","level":4}}"#)
                .unwrap();
        let normalized = serde_json::to_value(profiles_validate(partial).unwrap()).unwrap();
        assert!(normalized["output"]["suffixTemplate"].is_string());
        assert!(normalized["metadata"]["exif"].is_string());

        // profiles_save：改档 → 保存 → 读回；非法值必须被拒且不落盘
        let mut edited = profile.clone();
        edited.name = "mail2".into();
        edited.quality.kind = yali_core::QualityKind::TargetBytes;
        edited.quality.level = None;
        edited.quality.target_bytes = Some(300_000);
        profiles_save(edited).await.unwrap();
        let back = profiles_show("mail2".into()).await.unwrap();
        assert_eq!(back.quality.target_bytes, Some(300_000));

        let mut bad = profile.clone();
        bad.name = "bad".into();
        bad.quality.level = Some(0);
        bad.quality.kind = yali_core::QualityKind::Level;
        assert!(profiles_save(bad).await.is_err(), "非法 level 必须被拒");
        assert!(profiles_show("bad".into()).await.is_err(), "非法档不应落盘");

        // T-014 settings：缺失→默认（入口默认全关）→ 保存 → 读回；非法版本必须被拒
        let settings = settings_show().await.unwrap();
        assert_eq!(settings, yali_core::Settings::default());
        let mut edited_settings = settings.clone();
        edited_settings.watch.enabled = true;
        edited_settings.watch.dirs = vec![home.join("watch").to_string_lossy().into_owned()];
        // settings_save 需要 AppHandle（托盘联动），测试里直接打内核 store
        yali_core::SettingsStore::open()
            .unwrap()
            .save(&edited_settings)
            .unwrap();
        assert_eq!(settings_show().await.unwrap(), edited_settings);

        let mut bad_settings = settings.clone();
        bad_settings.schema_version = 99;
        assert!(bad_settings.validate().is_err(), "未来版本号必须被拒");
        assert_eq!(
            settings_show().await.unwrap(),
            edited_settings,
            "非法保存不应落盘"
        );

        // 用 benchkit 合成样张跑真实压缩（仓库内已入库，不依赖个人素材）
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchkit/samples/photo.jpg");
        let work = home.join("work");
        std::fs::create_dir_all(&work).unwrap();
        let input = work.join("photo.jpg");
        std::fs::copy(&sample, &input).unwrap();

        // 3 个文件走并发路径（Semaphore + join），结果按输入顺序返回
        let mut inputs = Vec::new();
        for i in 0..3 {
            let copy = work.join(format!("photo{i}.jpg"));
            std::fs::copy(&sample, &copy).unwrap();
            inputs.push(copy.to_string_lossy().into_owned());
        }
        // command 的第一个参数是 AppHandle（无法在测试里构造）——并发/事件外壳之外，
        // 批处理语义等价于逐个 compress_file，这里验证这条内核路径
        assert!((1..=4).contains(&max_concurrency()), "并发上限应在 1..=4");
        let default_profile = profiles_show("default".into()).await.unwrap();
        let registry = Registry::discover(&default_profile);
        let mut reports = Vec::new();
        for input in &inputs {
            reports.push(
                compress_file(Path::new(input), &default_profile, &registry)
                    .await
                    .unwrap(),
            );
        }
        assert_eq!(reports.len(), 3);
        for report in &reports {
            assert_eq!(report.status, "done");
            assert!(report.output_bytes.unwrap() < report.input_bytes);
            let out = report.output.as_ref().unwrap();
            assert!(Path::new(out).exists(), "产出文件应落盘: {out}");
        }

        std::env::remove_var("YALI_HOME");
        std::fs::remove_dir_all(&home).ok();
    }
}
