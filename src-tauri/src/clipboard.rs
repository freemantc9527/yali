//! T-016 剪贴板监听：NSPasteboard `changeCount` 轮询 → 取图 → 入队压缩 → 存到「下载」。
//!
//! 为什么不用 arboard：macOS 上 arboard 只给原始 RGBA，还得自己找 PNG 编码器；
//! 而 NSPasteboard 直接给 `public.png`（已经是编码后的字节，体积语义就是用户看到的体积）。
//! objc2 / objc2-app-kit 原本就在依赖树里（Tauri 带来），这里只是把它们变成直接依赖。

use std::time::Duration;

use yali_core::profile::{DirKind, Profile};

/// 轮询间隔：changeCount 是整数比较，700ms 一次对 CPU 基本无感
pub const POLL_INTERVAL: Duration = Duration::from_millis(700);

/// 剪贴板抽象（测试用 fake，生产用 NSPasteboard）
pub trait Pasteboard {
    fn change_count(&self) -> i64;
    /// 当前剪贴板里的图片（PNG 字节）；不是图片返回 None
    fn image_png(&self) -> Option<Vec<u8>>;
}

/// 去重状态机：只对「新的、够大的、内容没重复的」图片返回一次
pub struct ClipboardWatcher {
    pub min_bytes: u64,
    last_change: Option<i64>,
    last_digest: Option<u64>,
}

impl ClipboardWatcher {
    pub fn new(min_bytes: u64) -> Self {
        Self { min_bytes, last_change: None, last_digest: None }
    }

    /// 启动时对齐当前剪贴板：不把启动前就存在的内容当成新图片
    pub fn attach(&mut self, board: &dyn Pasteboard) {
        self.last_change = Some(board.change_count());
    }

    pub fn poll(&mut self, board: &dyn Pasteboard) -> Option<Vec<u8>> {
        let count = board.change_count();
        if self.last_change == Some(count) {
            return None;
        }
        self.last_change = Some(count);

        let png = board.image_png()?;
        if (png.len() as u64) <= self.min_bytes {
            return None; // 表情、小图标不值得压
        }
        let digest = fnv1a(&png);
        if self.last_digest == Some(digest) {
            return None; // 复制了同一张图（changeCount 变了，内容没变）
        }
        self.last_digest = Some(digest);
        Some(png)
    }
}

/// FNV-1a 64：只用来判「内容是否重复」，不需要密码学强度
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 剪贴板模式的输出档：存到「下载」、不带后缀、不写撤销清单
/// （剪贴板源图是临时文件，撤销没有意义；产品决定见 T-016 卡）
pub fn output_profile(base: Profile) -> Profile {
    let mut profile = base;
    profile.output.dir = DirKind::Downloads;
    profile.output.dir_path = None;
    profile.output.suffix_template = String::new();
    profile.output.undo_manifest = false;
    profile
}

pub fn notify(body: &str) {
    let escaped = body.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "display notification \"{escaped}\" with title \"鸭梨\" subtitle \"剪贴板图片已压好\""
    );
    let _ = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output();
}

#[cfg(target_os = "macos")]
mod system {
    use super::{tiff_to_png, Pasteboard};
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeTIFF};

    /// 系统剪贴板（必须在主线程调用，AppKit 不是线程安全的）
    pub struct SystemPasteboard;

    impl SystemPasteboard {
        pub fn new() -> Self {
            Self
        }
    }

    impl Pasteboard for SystemPasteboard {
        fn change_count(&self) -> i64 {
            NSPasteboard::generalPasteboard().changeCount() as i64
        }

        fn image_png(&self) -> Option<Vec<u8>> {
            let board = NSPasteboard::generalPasteboard();
            if let Some(data) = board.dataForType(unsafe { NSPasteboardTypePNG }) {
                return Some(data.to_vec());
            }
            // 有些 app（预览、部分浏览器）只放 TIFF：走 sips 转一次，别丢用户图
            let tiff = board.dataForType(unsafe { NSPasteboardTypeTIFF })?;
            tiff_to_png(&tiff.to_vec())
        }
    }
}

#[cfg(target_os = "macos")]
pub use system::SystemPasteboard;

/// TIFF → PNG（sips，系统自带；失败就放弃这张图，不猜）
#[cfg(target_os = "macos")]
pub fn tiff_to_png(tiff: &[u8]) -> Option<Vec<u8>> {
    let workspace = yali_core::data::TempDir::new().ok()?;
    let dir = workspace.path();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let src = dir.join(format!("{stamp}.tiff"));
    let dst = dir.join(format!("{stamp}.png"));
    std::fs::write(&src, tiff).ok()?;
    let status = std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "png"])
        .arg(&src)
        .arg("--out")
        .arg(&dst)
        .output()
        .ok()?;
    let png = if status.status.success() { std::fs::read(&dst).ok() } else { None };
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&dst);
    let _ = std::fs::remove_dir(&dir);
    png
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        count: std::cell::Cell<i64>,
        png: Option<Vec<u8>>,
    }

    impl Fake {
        fn new(png: Option<Vec<u8>>) -> Self {
            Self { count: std::cell::Cell::new(1), png }
        }
    }

    impl Pasteboard for Fake {
        fn change_count(&self) -> i64 {
            self.count.get()
        }
        fn image_png(&self) -> Option<Vec<u8>> {
            self.png.clone()
        }
    }

    #[test]
    fn attach_skips_existing_clipboard() {
        let board = Fake::new(Some(vec![7u8; 1024]));
        let mut watcher = ClipboardWatcher::new(0);
        watcher.attach(&board);
        assert!(watcher.poll(&board).is_none(), "启动前就在剪贴板里的图不应被当成新图");
    }

    #[test]
    fn same_change_count_is_not_reemitted() {
        let board = Fake::new(Some(vec![7u8; 1024]));
        let mut watcher = ClipboardWatcher::new(0);
        watcher.attach(&board);
        board.count.set(2);
        assert!(watcher.poll(&board).is_some());
        assert!(watcher.poll(&board).is_none(), "changeCount 没变就不要再压");
    }

    #[test]
    fn small_images_are_ignored() {
        let board = Fake::new(Some(vec![1u8; 100]));
        let mut watcher = ClipboardWatcher::new(64 * 1024);
        watcher.attach(&board);
        board.count.set(2);
        assert!(watcher.poll(&board).is_none(), "小于 minBytes 的图应跳过");
    }

    #[test]
    fn duplicate_content_with_new_count_is_ignored() {
        let mut board = Fake::new(Some(vec![3u8; 2048]));
        let mut watcher = ClipboardWatcher::new(0);
        watcher.attach(&board);
        board.count.set(2);
        assert!(watcher.poll(&board).is_some());
        board.count.set(3);
        assert!(watcher.poll(&board).is_none(), "同一张图复制两次只压一次");
        board.png = Some(vec![4u8; 2048]);
        board.count.set(4);
        assert!(watcher.poll(&board).is_some(), "内容变了要重新压");
    }

    #[test]
    fn empty_clipboard_advances_count_without_output() {
        let board = Fake::new(None);
        let mut watcher = ClipboardWatcher::new(0);
        watcher.attach(&board);
        board.count.set(2);
        assert!(watcher.poll(&board).is_none());
        // 之后放图但 count 不变（同一批写入）：内容已经在 changeCount 里消费掉了，不该重复处理
        assert!(watcher.poll(&board).is_none());
    }

    #[test]
    fn fnv1a_is_stable_and_content_sensitive() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_ne!(fnv1a(b"abc"), fnv1a(b"abd"));
    }

    #[test]
    fn output_profile_goes_to_downloads_without_suffix() {
        let raw = yali_core::store::FACTORY_PROFILES
            .iter()
            .find(|(name, _)| *name == "default")
            .unwrap()
            .1;
        let base: Profile = serde_json::from_str(raw).unwrap();
        let profile = output_profile(base);
        assert_eq!(profile.output.dir, DirKind::Downloads);
        assert_eq!(profile.output.suffix_template, "");
        assert!(!profile.output.undo_manifest);
        assert_eq!(profile.quality.level, Some(4), "剪贴板沿用默认档的质量参数");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn tiff_fallback_converts_to_png_via_sips() {
        let sample = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../benchkit/samples/photo.jpg");
        let dir = std::env::temp_dir().join(format!("yali-clip-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tiff = dir.join("probe.tiff");
        let status = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "tiff"])
            .arg(&sample)
            .arg("--out")
            .arg(&tiff)
            .output()
            .unwrap();
        assert!(status.status.success(), "sips 造 TIFF 失败");
        let bytes = std::fs::read(&tiff).unwrap();
        let png = tiff_to_png(&bytes).expect("TIFF 应能转成 PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "输出必须是 PNG 魔数");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
