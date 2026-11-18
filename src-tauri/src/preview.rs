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

pub async fn generate(path: String, max_edge: u32, cache: PathBuf) -> Result<String, String> {
    if ![0, 192, 1600].contains(&max_edge) {
        return Err("不支持的预览尺寸".into());
    }
    let _permit = PREVIEW_LIMIT
        .get_or_init(|| tokio::sync::Semaphore::new(2))
        .acquire()
        .await
        .map_err(|e| e.to_string())?;
    let source = std::fs::canonicalize(&path).map_err(|e| e.to_string())?;
    let metadata = std::fs::metadata(&source).map_err(|e| e.to_string())?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hash);
    metadata.len().hash(&mut hash);
    metadata.modified().ok().hash(&mut hash);
    max_edge.hash(&mut hash);
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let target = cache.join(format!("{:016x}.png", hash.finish()));
    if target.is_file() {
        return Ok(target.to_string_lossy().into_owned());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = cache.join(format!("preview-{}-{stamp}.png", std::process::id()));
    let mut command = tokio::process::Command::new("/usr/bin/sips");
    command.args(["-s", "format", "png"]);
    if max_edge > 0 {
        command.args(["-Z", &max_edge.to_string()]);
    }
    let output = command
        .arg(&source)
        .arg("--out")
        .arg(&temporary)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() || !temporary.is_file() {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!(
            "此格式暂不可预览：{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    std::fs::rename(&temporary, &target).map_err(|e| e.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn image_preview(
    app: tauri::AppHandle,
    path: String,
    max_edge: u32,
) -> Result<String, String> {
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("previews");
    generate(path, max_edge, cache).await
}

/// 仅清理本应用 previews 下超过一天的缓存；不递归、不跟随目录。
pub fn prune(cache: &Path) {
    let Ok(entries) = std::fs::read_dir(cache) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let expired = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|m| m.elapsed().ok())
                .map(|age| age.as_secs() > 86400)
                .unwrap_or(false);
            if expired {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn inspection_is_per_item_and_preview_preserves_source() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchkit/samples/photo.jpg");
        let source = sample.to_string_lossy().into_owned();
        let info = inspect_images(vec![source.clone(), "/missing.png".into()]).await;
        assert!(info[0].error.is_none());
        assert!(info[0].width.unwrap() > 0);
        assert!(info[1].error.is_some());
        let original = std::fs::read(&sample).unwrap();
        let cache = std::env::temp_dir().join(format!("yali-preview-test-{}", std::process::id()));
        assert!(generate(source.clone(), 400, cache.clone()).await.is_err());
        let preview = generate(source.clone(), 192, cache.clone()).await.unwrap();
        assert_eq!(generate(source, 192, cache.clone()).await.unwrap(), preview);
        assert_eq!(std::fs::read(sample).unwrap(), original);
        let (width, height) = dimensions(Path::new(&preview)).await;
        assert!(width.unwrap() <= 192 && height.unwrap() <= 192);
        std::fs::remove_dir_all(cache).ok();
    }
}
