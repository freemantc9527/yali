//! UndoManifest：`.yali-undo.json` 记录（input, output）对，`yali undo` / GUI ⌘Z 消费。
//! 还原规则：input 仍在 → 删除产出文件；input 已丢失 → 产出文件改名回 input（尽力还原）。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const MANIFEST_NAME: &str = ".yali-undo.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoEntry {
    pub input: String,
    pub output: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    entries: Vec<UndoEntry>,
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join(MANIFEST_NAME)
}

fn load(dir: &Path) -> Manifest {
    std::fs::read_to_string(manifest_path(dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn store(dir: &Path, manifest: &Manifest) -> Result<()> {
    let path = manifest_path(dir);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(manifest)? + "\n")?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// 追加一条（input, output）记录
pub fn append_entry(dir: &PathBuf, input: &Path, output: &Path) -> Result<()> {
    let mut manifest = load(dir);
    manifest.entries.push(UndoEntry {
        input: input.to_string_lossy().into(),
        output: output.to_string_lossy().into(),
    });
    store(dir, &manifest)
}

/// 还原结果
#[derive(Debug, Default)]
pub struct RestoreReport {
    pub removed: usize,
    pub restored_from_output: usize,
    pub missing: usize,
}

/// 按清单还原：input 在 → 删产出；input 丢 → 产出改名回 input
pub fn restore(dir: &Path) -> Result<RestoreReport> {
    let manifest = load(dir);
    let mut report = RestoreReport::default();
    for entry in &manifest.entries {
        let input = PathBuf::from(&entry.input);
        let output = PathBuf::from(&entry.output);
        match (input.exists(), output.exists()) {
            (true, true) => {
                std::fs::remove_file(&output)
                    .with_context(|| format!("删除产出失败: {}", output.display()))?;
                report.removed += 1;
            }
            (false, true) => {
                std::fs::rename(&output, &input)
                    .with_context(|| format!("还原文件失败: {} → {}", output.display(), input.display()))?;
                report.restored_from_output += 1;
            }
            _ => report.missing += 1,
        }
    }
    let _ = std::fs::remove_file(manifest_path(dir));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("yali-undo-test-{}-{}-{nanos}", tag, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn undo_manifest_roundtrip_and_restore() {
        let dir = temp_dir("restore");
        let input = dir.join("photo.jpg");
        let output = dir.join("photo-default.jpg");
        std::fs::write(&input, b"original-bytes").unwrap();
        std::fs::write(&output, b"compressed").unwrap();

        append_entry(&dir, &input, &output).unwrap();

        // 清单落盘且可读回
        let raw = std::fs::read_to_string(dir.join(MANIFEST_NAME)).unwrap();
        let m: Manifest = serde_json::from_str(&raw).unwrap();
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].input, input.to_string_lossy());

        let report = restore(&dir).unwrap();
        assert_eq!(report.removed, 1);
        assert!(!output.exists(), "产出文件应被删除");
        assert!(input.exists(), "原文件保持不动");
        assert!(!dir.join(MANIFEST_NAME).exists(), "还原后清单应移除");
        assert_eq!(std::fs::read(&input).unwrap(), b"original-bytes");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_restores_when_input_was_lost() {
        let dir = temp_dir("lost");
        let input = dir.join("gone.png");
        let output = dir.join("gone-default.png");
        std::fs::write(&output, b"compressed-png").unwrap();
        append_entry(&dir, &input, &output).unwrap();

        let report = restore(&dir).unwrap();
        assert_eq!(report.restored_from_output, 1);
        assert!(!output.exists());
        assert_eq!(std::fs::read(&input).unwrap(), b"compressed-png");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_counts_missing_entries() {
        let dir = temp_dir("missing");
        let input = dir.join("a.jpg");
        let output = dir.join("a-default.jpg");
        std::fs::write(&input, b"x").unwrap(); // output 不存在
        append_entry(&dir, &input, &output).unwrap();
        let report = restore(&dir).unwrap();
        assert_eq!(report.missing, 1);
        assert!(input.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn undo_manifest_appends_across_batches() {
        let dir = temp_dir("append");
        let a = dir.join("a.jpg");
        let b = dir.join("b.jpg");
        append_entry(&dir, &a, &dir.join("a-default.jpg")).unwrap();
        append_entry(&dir, &b, &dir.join("b-default.jpg")).unwrap();
        let m: Manifest = serde_json::from_str(&std::fs::read_to_string(dir.join(MANIFEST_NAME)).unwrap()).unwrap();
        assert_eq!(m.entries.len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }
}
