//! 引擎层：子进程引擎的识别 / 路径解析 / 参数映射 / 注册表。
//!
//! Phase A 全部走子进程（Brewfile 钉版，GPL 边界干净）；Phase B 库内链接另行开卡。
//! 路径解析顺序：`YALI_ENGINES_PATH`（冒号分隔）→ `PATH` → `(brew --prefix <pkg>)/bin`。
//! **mozjpeg 在 keg-only 路径**，PATH 里的 cjpeg 是 libjpeg-turbo 李鬼（坑账本）。

pub mod mapping;
pub mod registry;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 子进程引擎
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineKind {
    Mozjpeg,   // cjpeg / jpegtran（keg-only）
    Pngquant,  // PNG 有损量化
    Oxipng,    // PNG 无损优化
    Gifsicle,  // GIF
    Gif2Webp,  // 动图 webp（brew webp 包）
    Cwebp,     // WebP
    Avifenc,   // AVIF
    Cjxl,      // JXL（无损包裹 / 有损）
    Svgo,      // SVG（node 安装，可能缺席）
    Vips,      // 缩放 / 转码主力（冷启动 ~5s → 预热是设计要求）
    Sips,      // macOS 原生（HEIC 硬编等）
}

impl EngineKind {
    /// 二进制名与 brew 包名
    pub fn spec(self) -> (&'static str, &'static str) {
        match self {
            EngineKind::Mozjpeg => ("cjpeg", "mozjpeg"),
            EngineKind::Pngquant => ("pngquant", "pngquant"),
            EngineKind::Oxipng => ("oxipng", "oxipng"),
            EngineKind::Gifsicle => ("gifsicle", "gifsicle"),
            EngineKind::Gif2Webp => ("gif2webp", "webp"),
            EngineKind::Cwebp => ("cwebp", "webp"),
            EngineKind::Avifenc => ("avifenc", "libavif"),
            EngineKind::Cjxl => ("cjxl", "jpeg-xl"),
            EngineKind::Svgo => ("svgo", "svgo"),
            EngineKind::Vips => ("vipsthumbnail", "vips"),
            EngineKind::Sips => ("sips", "sips"),
        }
    }

    /// mozjpeg 生态的第二个二进制（jpegtran 走同一 keg 路径）
    pub fn sibling_binary(self, name: &str) -> Option<PathBuf> {
        if self == EngineKind::Mozjpeg {
            return self.resolve_binary(name);
        }
        None
    }

    fn resolve_binary(self, binary: &str) -> Option<PathBuf> {
        let (_, pkg) = self.spec();
        // YALI_ENGINES_PATH 优先（冒号分隔，逐目录找目标二进制）
        if let Ok(extra) = std::env::var("YALI_ENGINES_PATH") {
            for dir in extra.split(':').filter(|s| !s.is_empty()) {
                let candidate = Path::new(dir).join(binary);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        // PATH（mozjpeg 特判：PATH 里的 cjpeg/jpegtran 可能是 libjpeg-turbo 李鬼，
        // 只认 keg-only 目录 /mozjpeg/ 下的）
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':').filter(|s| !s.is_empty()) {
                let candidate = Path::new(dir).join(binary);
                if candidate.is_file()
                    && (self != EngineKind::Mozjpeg || candidate.to_string_lossy().contains("/mozjpeg/"))
                {
                    return Some(candidate);
                }
            }
        }
        // Homebrew 标准目录兜底：Finder Services / launchd 上下文的 PATH 只有
        // /usr/bin:/bin，不能假设 /opt/homebrew/bin 在 PATH 里
        for dir in ["/opt/homebrew/bin", "/usr/local/bin"] {
            let candidate = Path::new(dir).join(binary);
            if candidate.is_file()
                && (self != EngineKind::Mozjpeg || candidate.to_string_lossy().contains("/mozjpeg/"))
            {
                return Some(candidate);
            }
        }
        // Homebrew keg 路径兜底（mozjpeg 这类 keg-only 必须靠它）
        if let Some(prefix) = keg_prefix(pkg) {
            let candidate = prefix.join("bin").join(binary);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    /// 解析引擎主二进制路径（找不到返回 None，doctor 会如实上报）
    pub fn resolve(self) -> Option<PathBuf> {
        let (binary, _) = self.spec();
        self.resolve_binary(binary)
    }
}

/// Homebrew keg 目录（mozjpeg 这类 keg-only 靠它）。
/// **不调用 `brew --prefix`**：HOME 陌生时 brew 会先拉 API 数据，网络不畅会挂死
/// （实测在服务上下文可挂 3 分钟+）。Homebrew 的 opt/<pkg> 是稳定布局，直接探目录。
fn keg_prefix(pkg: &str) -> Option<PathBuf> {
    ["/opt/homebrew", "/usr/local"]
        .iter()
        .map(|base| Path::new(base).join("opt").join(pkg))
        .find(|p| p.is_dir())
}

/// 引擎可用性探测结果（doctor 输出）
#[derive(Debug, Clone)]
pub struct EngineProbe {
    pub kind: EngineKind,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

impl EngineProbe {
    pub fn available(&self) -> bool {
        self.path.is_some()
    }
}

impl EngineKind {
    /// 探测路径 + 版本（版本尽力而为，各引擎参数不同）
    pub fn probe(self) -> EngineProbe {
        let path = self.resolve();
        let version = path.as_ref().and_then(|p| version_of(self, p));
        EngineProbe { kind: self, path, version }
    }
}

fn version_of(kind: EngineKind, path: &Path) -> Option<String> {
    let (args, use_stderr) = match kind {
        EngineKind::Mozjpeg => (["-version"], true), // cjpeg -version 走 stderr
        EngineKind::Cwebp | EngineKind::Gif2Webp => (["-version"], false),
        EngineKind::Sips => return None,             // sips 无版本参数
        _ => (["--version"], false),
    };
    let out = std::process::Command::new(path).args(args).output().ok()?;
    let text = if use_stderr {
        String::from_utf8_lossy(&out.stderr)
    } else {
        String::from_utf8_lossy(&out.stdout)
    };
    let first = text.lines().next().unwrap_or("").trim().to_string();
    if first.is_empty() { None } else { Some(first) }
}

/// 运行引擎子进程（tokio，捕获 stdout/stderr）
pub(crate) async fn run_engine(program: &Path, args: &[String]) -> Result<std::process::Output> {
    tokio::process::Command::new(program)
        .args(args)
        .output()
        .await
        .with_context(|| format!("执行引擎失败: {} {:?}", program.display(), args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// env 是进程级全局：所有改 env 的测试共用这把锁
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn engine_mozjpeg_resolution_rejects_libjpeg_turbo_cjpeg() {
        // 本机 PATH 里 /opt/homebrew/bin/cjpeg 是 libjpeg-turbo 的李鬼：
        // 解析结果必须落在 /mozjpeg/ keg 路径里（或找不到，绝不能返回李鬼）
        let path = EngineKind::Mozjpeg.resolve();
        if let Some(p) = path {
            assert!(
                p.to_string_lossy().contains("/mozjpeg/"),
                "cjpeg 必须走 mozjpeg keg 路径，得到 {p:?}"
            );
        }
    }

    #[test]
    fn engine_yali_engines_path_takes_priority() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("yali-engines-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pngquant"), "#!/bin/sh\n").unwrap();
        std::env::set_var("YALI_ENGINES_PATH", &dir);
        let resolved = EngineKind::Pngquant.resolve();
        std::env::remove_var("YALI_ENGINES_PATH");
        assert_eq!(resolved, Some(dir.join("pngquant")));
    }

    #[test]
    fn engine_resolves_without_brew_in_path() {
        // Finder Services / launchd 的 PATH 只有 /usr/bin:/bin：Homebrew 目录必须自带兜底
        let _guard = ENV_LOCK.lock().unwrap();
        let original = std::env::var("PATH").ok();
        std::env::set_var("PATH", "/usr/bin:/bin");
        let path = EngineKind::Oxipng.resolve();
        match original {
            Some(v) => std::env::set_var("PATH", v),
            None => std::env::remove_var("PATH"),
        }
        let path = path.expect("PATH 缺失时 oxipng 仍应通过 Homebrew 兜底解析到");
        assert!(
            path.starts_with("/opt/homebrew") || path.starts_with("/usr/local"),
            "兜底路径应指向 Homebrew: {path:?}"
        );
    }

    #[test]
    fn engine_probe_reports_availability() {
        let probe = EngineKind::Oxipng.probe();
        assert!(probe.available(), "本机 oxipng 应可用（Brewfile 已装）");
    }
}
