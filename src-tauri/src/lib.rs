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
async fn profiles_save(profile: Profile) -> Result<(), String> {
    profile.validate().map_err(err)?;
    let store = ProfileStore::open().map_err(err)?;
    store.save(&profile).map_err(err)
}

#[tauri::command]
async fn compress(paths: Vec<String>, profile: String) -> Result<Vec<CompressReport>, String> {
    let profile = ProfileStore::open().map_err(err)?.load(&profile).map_err(err)?;
    let registry = Registry::discover(&profile);
    registry.warmup().await; // vips 冷启动对冲（坑账本）
    let mut reports = Vec::with_capacity(paths.len());
    for path in paths {
        let report = compress_file(Path::new(&path), &profile, &registry)
            .await
            .map_err(err)?;
        reports.push(report);
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

        let reports = compress(vec![input.to_string_lossy().into()], "default".into())
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].status, "done");
        assert!(reports[0].output_bytes.unwrap() < reports[0].input_bytes);
        let out = reports[0].output.as_ref().unwrap();
        assert!(Path::new(out).exists(), "产出文件应落盘: {out}");

        std::env::remove_var("YALI_HOME");
        std::fs::remove_dir_all(&home).ok();
    }
}
