//! yali — 鸭梨 CLI。所有入口（workflow/GUI/剪贴板）共用的内核门面。
//! T-003：compress/convert/lossless/undo/doctor 真实现（tokio 并发 ≤ min(cores-1, 4)）。

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::sync::Semaphore;
use yali_core::engines::registry::Registry;
use yali_core::pipeline::{compress_file, InputFormat};
use yali_core::profile::{OutputFormat, Profile, QualityKind, ResizeMode};
use yali_core::undo;
use yali_core::ProfileStore;

#[derive(Parser)]
#[command(name = "yali", version, about = "鸭梨 Yali — 让图片没有压力")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 压缩图片（右键「压一下」走的就是这条路径）
    Compress {
        files: Vec<String>,
        #[arg(short, long, default_value = "default")]
        profile: String,
        /// 只输出 JSON 摘要，不打印过程信息（workflow 用）
        #[arg(long)]
        quiet: bool,
        /// 结束时发一条聚合通知（Finder 右键「压一下」用）
        #[arg(long)]
        notify: bool,
    },
    /// 格式转换
    Convert {
        files: Vec<String>,
        #[arg(short = 't', long = "to")]
        to: String,
    },
    /// JXL 无损包裹归档（可逆）
    Lossless { files: Vec<String> },
    /// 按撤销清单还原
    Undo { dir: String },
    /// Profile 管理
    Profiles {
        #[command(subcommand)]
        action: ProfilesAction,
    },
    /// 环境自检（引擎路径/版本，报 issue 时先跑这个）
    Doctor,
}

#[derive(Subcommand)]
enum ProfilesAction {
    /// 列出磁盘上所有档（首启自动播种五档出厂 fixtures）
    List,
    /// 从磁盘读取指定档，输出 JSON
    Show { name: String },
}

fn parse_format(s: &str) -> Result<OutputFormat> {
    match s.to_lowercase().as_str() {
        "jpeg" | "jpg" => Ok(OutputFormat::Jpeg),
        "png" => Ok(OutputFormat::Png),
        "webp" => Ok(OutputFormat::Webp),
        "avif" => Ok(OutputFormat::Avif),
        "heic" => Ok(OutputFormat::Heic),
        "jxl" => Ok(OutputFormat::Jxl),
        "gif" => Ok(OutputFormat::Gif),
        "svg" => Ok(OutputFormat::Svg),
        other => Err(anyhow::anyhow!("未知目标格式 '{other}'")),
    }
}

/// 按名字加载 profile（磁盘读取，不再内置硬编码）
fn load_profile(name: &str) -> Result<Profile> {
    let store = ProfileStore::open()?;
    store.load(name)
}

fn max_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(2)
        .min(4)
}

/// 批处理汇总（聚合通知的依据；三态覆盖：done / larger_output / failed）
#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct BatchSummary {
    done: usize,
    larger: usize,
    failed: usize,
    saved_bytes: i64,
}

fn summarize(reports: &[yali_core::pipeline::CompressReport], failed: usize) -> BatchSummary {
    let mut s = BatchSummary { failed, ..Default::default() };
    for r in reports {
        if r.status == "done" {
            s.done += 1;
            s.saved_bytes += r.saved_bytes.unwrap_or(0);
        } else if r.status == "larger_output" {
            s.larger += 1;
        }
    }
    s
}

/// 人类可读体积（通知文案用）
fn human_bytes(bytes: i64) -> String {
    let b = bytes.max(0) as f64;
    if b < 1024.0 {
        format!("{b:.0} B")
    } else if b < 1024.0 * 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", b / (1024.0 * 1024.0 * 1024.0))
    }
}

/// 聚合通知文案：多选只发一条
fn notification_text(s: &BatchSummary) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if s.done > 0 {
        parts.push(format!("{} 张省了 {}", s.done, human_bytes(s.saved_bytes)));
    }
    if s.larger > 0 {
        if s.done > 0 {
            parts.push(format!("{} 张压不动已保持原样", s.larger));
        } else {
            parts.push(format!("{} 张压不动，已保持原样", s.larger));
        }
    }
    if s.failed > 0 {
        parts.push(format!("{} 张失败", s.failed));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("，"))
    }
}

/// osascript 发通知（失败不影响压缩结果）
fn notify(text: &str) {
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{escaped}\" with title \"鸭梨\" subtitle \"图片压完了\""
    );
    let _ = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output();
}

async fn run_batch(
    files: Vec<String>,
    profile: Profile,
    quiet: bool,
    notify_flag: bool,
) -> Result<()> {
    let registry = Arc::new(Registry::discover(&profile));
    registry.warmup().await; // vips 冷启动 ~5s：预热是设计要求（坑账本）

    let semaphore = Arc::new(Semaphore::new(max_concurrency()));
    let mut handles = Vec::new();
    for file in files {
        let permit = semaphore.clone().acquire_owned().await?;
        let registry = registry.clone();
        let profile = profile.clone();
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let path = PathBuf::from(&file);
            let result = compress_file(&path, &profile, &registry).await;
            (file, result)
        }));
    }

    let mut reports = Vec::new();
    let mut failures = 0usize;
    for handle in handles {
        let (file, result) = handle.await?;
        match result {
            Ok(report) => {
                println!("{}", serde_json::to_string(&report)?);
                reports.push(report);
            }
            Err(e) => {
                failures += 1;
                if !quiet {
                    eprintln!("✗ {file}: {e:#}");
                } else {
                    eprintln!("{file}: {e:#}");
                }
            }
        }
    }

    let summary = summarize(&reports, failures);
    if notify_flag {
        if let Some(text) = notification_text(&summary) {
            notify(&text);
        }
    }
    if failures > 0 {
        anyhow::bail!("{failures} 个文件处理失败");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Compress { files, profile, quiet, notify } => {
            let profile = load_profile(&profile)?;
            run_batch(files, profile, quiet, notify).await
        }
        Cmd::Convert { files, to } => {
            let mut profile = load_profile("default")?;
            profile.format.output = parse_format(&to)?;
            run_batch(files, profile, false, false).await
        }
        Cmd::Lossless { files } => {
            let mut profile = load_profile("default")?;
            profile.quality.kind = QualityKind::Lossless;
            profile.format.output = OutputFormat::Jxl; // cjxl --lossless_jpeg=1 包裹
            profile.resize.mode = ResizeMode::None;
            run_batch(files, profile, false, false).await
        }
        Cmd::Undo { dir } => {
            let report = undo::restore(&PathBuf::from(&dir))?;
            println!(
                "还原完成：删除产出 {}，从产出回补 {}，缺失 {}",
                report.removed, report.restored_from_output, report.missing
            );
            Ok(())
        }
        Cmd::Profiles { action } => match action {
            ProfilesAction::List => {
                let store = ProfileStore::open()?;
                for name in store.list()? {
                    println!("{name}");
                }
                Ok(())
            }
            ProfilesAction::Show { name } => {
                let store = ProfileStore::open()?;
                let profile = store.load(&name)?;
                println!("{}", serde_json::to_string_pretty(&profile)?);
                Ok(())
            }
        },
        Cmd::Doctor => {
            let profile = Profile::default();
            let registry = Registry::discover(&profile);
            println!("yali v{} · 引擎自检", yali_core::CORE_VERSION);
            let mut kinds: Vec<_> = registry.probes.keys().copied().collect();
            kinds.sort_by_key(|k| format!("{k:?}"));
            for kind in kinds {
                let probe = &registry.probes[&kind];
                let status = if probe.available() { "✓" } else { "✗ 缺失" };
                let path = probe
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let version = probe.version.clone().unwrap_or_default();
                println!("{status} {kind:?}: {path} {version}");
            }
            // 输入格式嗅探可用性提示
            println!();
            println!("支持输入：{:?}", [
                InputFormat::Jpeg, InputFormat::Png, InputFormat::Gif, InputFormat::Webp,
                InputFormat::Svg, InputFormat::Avif, InputFormat::Heic, InputFormat::Jxl,
            ]);
            Ok(())
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use yali_core::pipeline::CompressReport;

    fn report(status: &'static str, saved: Option<i64>) -> CompressReport {
        CompressReport {
            input: "a.jpg".into(),
            output: Some("a-default.jpg".into()),
            input_bytes: 1_000_000,
            output_bytes: Some(440_000),
            saved_bytes: saved,
            saved_pct: saved.map(|s| s as f64 / 1_000_000.0 * 100.0),
            engine: "mozjpeg".into(),
            elapsed_ms: 90,
            status,
        }
    }

    #[test]
    fn summary_counts_three_states_and_savings() {
        let reports = vec![
            report("done", Some(560_000)),
            report("done", Some(440_000)),
            report("larger_output", None),
        ];
        let s = summarize(&reports, 1);
        assert_eq!(s.done, 2);
        assert_eq!(s.larger, 1);
        assert_eq!(s.failed, 1);
        assert_eq!(s.saved_bytes, 1_000_000);
    }

    #[test]
    fn notification_text_aggregates_one_line() {
        assert_eq!(
            notification_text(&BatchSummary { done: 3, larger: 0, failed: 0, saved_bytes: 5_000_000 }),
            Some("3 张省了 4.8 MB".to_string())
        );
        assert_eq!(
            notification_text(&BatchSummary { done: 2, larger: 1, failed: 0, saved_bytes: 1_000 }),
            Some("2 张省了 1000 B，1 张压不动已保持原样".to_string())
        );
        assert_eq!(
            notification_text(&BatchSummary { done: 0, larger: 2, failed: 0, saved_bytes: 0 }),
            Some("2 张压不动，已保持原样".to_string())
        );
        assert_eq!(notification_text(&BatchSummary::default()), None);
    }

    #[test]
    fn human_bytes_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1000), "1000 B");
        assert_eq!(human_bytes(5_000_000), "4.8 MB");
    }
}
