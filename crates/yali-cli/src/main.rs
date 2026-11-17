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

async fn run_batch(
    files: Vec<String>,
    profile: Profile,
    quiet: bool,
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

    let mut failures = 0usize;
    for handle in handles {
        let (file, result) = handle.await?;
        match result {
            Ok(report) => {
                println!("{}", serde_json::to_string(&report)?);
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
    if failures > 0 {
        anyhow::bail!("{failures} 个文件处理失败");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Compress { files, profile, quiet } => {
            let profile = load_profile(&profile)?;
            run_batch(files, profile, quiet).await
        }
        Cmd::Convert { files, to } => {
            let mut profile = load_profile("default")?;
            profile.format.output = parse_format(&to)?;
            run_batch(files, profile, false).await
        }
        Cmd::Lossless { files } => {
            let mut profile = load_profile("default")?;
            profile.quality.kind = QualityKind::Lossless;
            profile.format.output = OutputFormat::Jxl; // cjxl --lossless_jpeg=1 包裹
            profile.resize.mode = ResizeMode::None;
            run_batch(files, profile, false).await
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
