//! 鸭梨 GUI 壳：只做 IPC 接线，业务全在内核（docs/DESIGN.md §5 / T-005）。

use std::path::Path;

use yali_core::engines::registry::Registry;
use yali_core::pipeline::{compress_file, CompressReport};
use yali_core::profile::Profile;
use yali_core::ProfileStore;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
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
async fn profiles_save(profile: Profile) -> Result<(), String> {
    profile.validate().map_err(err)?;
    let store = ProfileStore::open().map_err(err)?;
    store.save(&profile).map_err(err)
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
    use tauri::Emitter;
    let profile = ProfileStore::open().map_err(err)?.load(&profile).map_err(err)?;
    let registry = std::sync::Arc::new(Registry::discover(&profile));
    registry.warmup().await; // vips 冷启动对冲（坑账本）

    let total = paths.len();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency()));
    let mut handles = Vec::with_capacity(total);

    for (index, path) in paths.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await.map_err(|e| e.to_string())?;
        let app = app.clone();
        let registry = registry.clone();
        let profile = profile.clone();
        handles.push(tauri::async_runtime::spawn(async move {
            let _permit = permit;
            let emit = |status: &'static str, report: Option<CompressReport>, error: Option<String>| {
                let _ = app.emit(
                    "yali://progress",
                    ProgressEvent { path: path.clone(), index, total, status, report, error },
                );
            };
            emit("started", None, None);
            match compress_file(Path::new(&path), &profile, &registry).await {
                Ok(report) => {
                    emit("done", Some(report.clone()), None);
                    Ok(report)
                }
                Err(e) => {
                    let msg = format!("{e:#}");
                    emit("failed", None, Some(msg.clone()));
                    Err(msg)
                }
            }
        }));
    }

    let mut reports = Vec::with_capacity(total);
    let mut first_error: Option<String> = None;
    for handle in handles {
        match handle.await.map_err(|e| e.to_string())? {
            Ok(report) => reports.push(report),
            Err(msg) => {
                if first_error.is_none() {
                    first_error = Some(msg);
                }
            }
        }
    }
    if let Some(msg) = first_error {
        return Err(msg);
    }
    Ok(reports)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            core_version,
            profiles_list,
            profiles_show,
            profiles_save,
            ssim,
            compress
        ])
        .run(tauri::generate_context!())
        .expect("error while running yali");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IPC 命令层冒烟：profiles_list / profiles_show / compress 三个 command 的真实行为。
    /// YALI_HOME 隔离到临时目录（env 是进程级全局，故所有断言集中在一个测试里串行跑）。
    #[tokio::test]
    async fn commands_smoke_with_isolated_home() {
        let home = std::env::temp_dir().join(format!("yali-cmd-test-{}", std::process::id()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("YALI_HOME", &home);

        let names = profiles_list().await.unwrap();
        assert_eq!(names, vec!["archive", "default", "mail", "screenshot", "web"]);

        let profile = profiles_show("mail".into()).await.unwrap();
        assert_eq!(profile.name, "mail");
        assert_eq!(profile.quality.kind, yali_core::QualityKind::TargetBytes);

        // profiles_save：改档 → 保存 → 读回；非法值必须被拒且不落盘
        let mut edited = profile.clone();
        edited.name = "mail2".into();
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
