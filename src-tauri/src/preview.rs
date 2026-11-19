//! 只生成应用缓存副本，绝不修改用户图片。
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tauri::Manager;

static PREVIEW_LIMIT: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    path: String,
    canonical_path: Option<String>,
    bytes: Option<u64>,
    format: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    error: Option<String>,
}

async fn dimensions(path: &Path) -> (Option<u32>, Option<u32>) {
    let output = tokio::process::Command::new("/usr/bin/sips")
        .args(["-g", "pixelWidth", "-g", "pixelHeight"])
        .arg(path)
        .output()
        .await;
    let Ok(output) = output else {
        return (None, None);
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let value = |key: &str| {
        text.lines()
            .find_map(|line| line.trim().strip_prefix(key)?.trim().parse::<u32>().ok())
    };
    (value("pixelWidth:"), value("pixelHeight:"))
}

#[tauri::command]
pub async fn inspect_images(paths: Vec<String>) -> Vec<ImageInfo> {
    let mut results = Vec::new();
    for path in paths {
        let mut info = ImageInfo {
            path: path.clone(),
            canonical_path: None,
            bytes: None,
            format: None,
            width: None,
            height: None,
            error: None,
        };
        let result = (|| -> anyhow::Result<PathBuf> {
            anyhow::ensure!(Path::new(&path).is_absolute(), "图片必须使用绝对路径");
            let canonical = std::fs::canonicalize(&path)?;
            info.canonical_path = Some(canonical.to_string_lossy().into_owned());
            let metadata = std::fs::metadata(&canonical)?;
            anyhow::ensure!(metadata.is_file(), "请选择图片文件，不支持目录递归导入");
            info.bytes = Some(metadata.len());
            info.format = Some(yali_core::pipeline::sniff(&canonical)?.to_string());
            Ok(canonical)
        })();
        match result {
            Ok(canonical) => (info.width, info.height) = dimensions(&canonical).await,
            Err(error) => info.error = Some(format!("{error:#}")),
        }
        results.push(info);
    }
    results
}

#[derive(Default)]
pub struct PreviewLeases(std::sync::Mutex<std::collections::HashMap<String, std::fs::File>>);

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    path: String,
    lease_id: String,
}

struct StagedPreview(PathBuf);
impl Drop for StagedPreview {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub async fn generate(
    path: String,
    max_edge: u32,
    cache: PathBuf,
    scope: &tauri::scope::fs::Scope,
) -> Result<(String, std::fs::File), String> {
    if ![0, 192].contains(&max_edge) {
        return Err("不支持的预览尺寸".into());
    }
    let _permit = PREVIEW_LIMIT
        .get_or_init(|| tokio::sync::Semaphore::new(2))
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    let source = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    let metadata = std::fs::metadata(&source).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("请选择图片文件".into());
    }
    let root = cache.parent().ok_or("无效缓存目录")?.to_path_buf();
    let guard_root = root.clone();
    let _guard = tokio::task::spawn_blocking(move || yali_core::cache::lock(&guard_root))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hash);
    metadata.len().hash(&mut hash);
    metadata.modified().ok().hash(&mut hash);
    max_edge.hash(&mut hash);
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    if cache.is_symlink() {
        return Err("预览目录不能是符号链接".into());
    }
    let target = cache.join(format!("{:016x}.png", hash.finish()));
    if target.is_symlink() { return Err("预览文件不能是符号链接".into()); }
    if !target.is_file() {
        // 生成期间 maintenance 锁阻止并发清理，取消/失败清理临时文件。
        let temporary = cache.join(format!("preview-{}.png", std::process::id()));
        let _staged = StagedPreview(temporary.clone());
        let mut command = tokio::process::Command::new("/usr/bin/sips");
        command.kill_on_drop(true).args(["-s", "format", "png"]);
        if max_edge > 0 {
            command.args(["-Z", &max_edge.to_string()]);
        }
        let result = command
            .arg(&source)
            .arg("--out")
            .arg(&temporary)
            .output()
            .await;
        match result {
            Ok(output) if output.status.success() && temporary.is_file() => {
                std::fs::rename(&temporary, &target).map_err(|e| e.to_string())?;
            }
            output => {
                let _ = std::fs::remove_file(&temporary);
                return Err(format!("此格式暂不可预览：{output:?}"));
            }
        }
    }
    let lease = std::fs::File::open(&target).map_err(|e| e.to_string())?;
    lease.lock_shared().map_err(|e| e.to_string())?;
    lease
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::now()))
        .map_err(|e| e.to_string())?;
    scope.allow_file(&target).map_err(|e| e.to_string())?;
    yali_core::cache::prune_locked(
        &root,
        false,
        yali_core::cache::LIMIT_BYTES,
        yali_core::cache::MAX_AGE,
        std::time::SystemTime::now(),
    )
    .map_err(|e| e.to_string())?;
    Ok((target.to_string_lossy().into_owned(), lease))
}

#[tauri::command]
pub async fn image_preview(
    app: tauri::AppHandle,
    path: String,
    max_edge: u32,
) -> Result<Preview, String> {
    let cache = yali_core::cache::root()
        .map_err(|e| e.to_string())?
        .join("previews");
    let (path, lease) = generate(path, max_edge, cache, &app.asset_protocol_scope()).await?;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let lease_id = NEXT
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .to_string();
    app.state::<PreviewLeases>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(lease_id.clone(), lease);
    Ok(Preview { path, lease_id })
}

#[tauri::command]
pub async fn release_preview(app: tauri::AppHandle, lease_id: String) -> Result<(), String> {
    app.state::<PreviewLeases>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .remove(&lease_id);
    maintain(false).await?;
    Ok(())
}

#[tauri::command]
pub fn image_source(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    yali_core::pipeline::sniff(&path).map_err(|e| e.to_string())?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

pub async fn maintain(clear: bool) -> Result<yali_core::cache::Status, String> {
    tokio::task::spawn_blocking(move || {
        yali_core::cache::prune_at(&yali_core::cache::root()?, clear)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cache_status() -> Result<yali_core::cache::Status, String> {
    tokio::task::spawn_blocking(|| yali_core::cache::status_at(&yali_core::cache::root()?))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cache_clear() -> Result<yali_core::cache::Status, String> {
    maintain(true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn inspection_is_per_item_and_preview_preserves_source() {
        let app = tauri::test::mock_app();
        let config: tauri::Config =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let scope =
            tauri::scope::fs::Scope::new(&app, &config.app.security.asset_protocol.scope).unwrap();
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchkit/samples/photo.jpg");
        let source = sample.to_string_lossy().into_owned();
        let info = inspect_images(vec![source.clone(), "/missing.png".into()]).await;
        assert!(info[0].error.is_none());
        assert!(info[0].width.unwrap() > 0);
        assert!(info[1].error.is_some());
        let original = std::fs::read(&sample).unwrap();
        let test_root =
            std::env::temp_dir().join(format!("yali-preview-test-{}", std::process::id()));
        let cache = test_root.join(".yali [test]/cache/previews");
        assert!(generate(source.clone(), 400, cache.clone(), &scope)
            .await
            .is_err());
        for edge in [192, 0] {
            let (preview, _lease) = generate(source.clone(), edge, cache.clone(), &scope)
                .await
                .unwrap();
            assert!(
                scope.is_allowed(&preview),
                "asset protocol denied {preview}"
            );
            // A fresh application scope must also grant an existing cache hit.
            let fresh =
                tauri::scope::fs::Scope::new(&app, &config.app.security.asset_protocol.scope)
                    .unwrap();
            assert!(!fresh.is_allowed(&preview));
            assert_eq!(
                generate(source.clone(), edge, cache.clone(), &fresh)
                    .await
                    .unwrap()
                    .0,
                preview
            );
            assert!(fresh.is_allowed(&preview));
            assert!(!fresh.is_allowed(cache.join("unrelated.png")));
        }
        let (preview, lease) = generate(source, 192, cache.clone(), &scope).await.unwrap();
        assert_eq!(std::fs::read(sample).unwrap(), original);
        let (width, height) = dimensions(Path::new(&preview)).await;
        assert!(width.unwrap() <= 192 && height.unwrap() <= 192);
        drop(lease);
        std::fs::remove_dir_all(test_root).ok();
    }
}
