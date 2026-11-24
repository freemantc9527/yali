//! 已构建 App 提供自包含图片引擎；开发态保留 Homebrew 查找。可选外部 FFmpeg 由 metrics 单独解析。
use std::path::{Path, PathBuf};

pub fn contents() -> Option<PathBuf> {
    contents_for(&std::env::current_exe().ok()?)
}

fn contents_for(executable: &Path) -> Option<PathBuf> {
    let executable = executable.canonicalize().ok()?;
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    (macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && contents.join("Resources/yali-runtime.json").is_file())
    .then(|| contents.to_path_buf())
}

/// macOS Contents 与 Windows ZIP 是不同布局；Services 仍只使用 contents()。
pub fn is_packaged() -> bool {
    contents().is_some() || windows_root().is_some()
}

fn windows_root() -> Option<PathBuf> {
    #[cfg(windows)]
    { zip_root_for(&std::env::current_exe().ok()?) }
    #[cfg(not(windows))]
    { None }
}

#[cfg(any(windows, test))]
fn zip_root_for(executable: &Path) -> Option<PathBuf> {
    let executable = executable.canonicalize().ok()?;
    let root = executable.parent()?;
    root.join("yali-runtime.json").exists().then(|| root.to_path_buf())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    target: String,
    tools: std::collections::BTreeMap<String, Tool>,
    files: std::collections::BTreeMap<String, String>,
}

#[derive(serde::Deserialize)]
struct Tool { path: String }

fn relative_file(root: &Path, raw: &str) -> anyhow::Result<PathBuf> {
    anyhow::ensure!(!raw.is_empty() && !raw.contains([':', '\\']) && !raw.starts_with('/'), "运行时路径必须是包内相对路径: {raw}");
    anyhow::ensure!(raw.split('/').all(|part| !matches!(part, "" | "." | "..")), "运行时路径越界: {raw}");
    let path = root.join(raw).canonicalize()?;
    anyhow::ensure!(path.starts_with(root.canonicalize()?), "运行时文件越界: {raw}");
    anyhow::ensure!(path.is_file(), "运行时文件缺失: {raw}");
    Ok(path)
}

fn read_manifest(root: &Path) -> anyhow::Result<Manifest> {
    use sha2::{Digest, Sha256};
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(root.join("yali-runtime.json"))?)?;
    anyhow::ensure!(manifest.schema_version == 1 && manifest.target == "x86_64-pc-windows-msvc", "Windows 运行时 schema 或架构不匹配");
    for name in ["vips", "vipsheader", "vipsthumbnail", "cjpeg", "jpegtran", "pngquant", "oxipng", "gifsicle", "cwebp", "gif2webp"] {
        let tool = manifest.tools.get(name).ok_or_else(|| anyhow::anyhow!("运行时缺少工具: {name}"))?;
        anyhow::ensure!(tool.path.ends_with(".exe") && manifest.files.contains_key(&tool.path), "工具未纳入清单: {name}");
    }
    for (raw, expected) in &manifest.files {
        let path = relative_file(root, raw)?;
        let mut file = std::fs::File::open(&path)?;
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut hash)?;
        anyhow::ensure!(format!("{:x}", hash.finalize()) == *expected, "运行时文件校验失败: {raw}");
        if raw.ends_with(".exe") || raw.ends_with(".dll") {
            use std::io::{Read, Seek, SeekFrom};
            file.rewind()?;
            let mut dos = [0; 64]; file.read_exact(&mut dos)?;
            anyhow::ensure!(&dos[..2] == b"MZ", "无效 PE: {raw}");
            file.seek(SeekFrom::Start(u32::from_le_bytes(dos[60..64].try_into()?) as u64))?;
            let mut pe = [0; 6]; file.read_exact(&mut pe)?;
            anyhow::ensure!(&pe[..4] == b"PE\0\0" && pe[4..] == [0x64, 0x86], "运行时要求 x64 PE: {raw}");
        }
    }
    Ok(manifest)
}

// 对当前包只校验一次；不会缓存开发环境或在运行时搜索 CWD。
fn windows_manifest() -> anyhow::Result<Option<&'static Manifest>> {
    static MANIFEST: std::sync::OnceLock<Result<Manifest, String>> = std::sync::OnceLock::new();
    let Some(root) = windows_root() else { return Ok(None); };
    match MANIFEST.get_or_init(|| read_manifest(&root).map_err(|e| format!("Windows 图片运行时损坏或缺件: {e:#}"))) {
        Ok(manifest) => Ok(Some(manifest)),
        Err(error) => anyhow::bail!("{error}"),
    }
}

pub fn validate() -> anyhow::Result<()> {
    windows_manifest()?;
    Ok(())
}

pub fn bundled_binary(name: &str) -> Option<PathBuf> {
    if let Some(root) = windows_root() {
        let manifest = windows_manifest().ok()??;
        return relative_file(&root, &manifest.tools.get(name)?.path).ok();
    }
    binary_in(&contents()?, name)
}

pub fn command(program: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.envs(command_environment(program));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

pub fn async_command(program: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::from(command(program));
    command.kill_on_drop(true);
    command
}

/// 由 App 主程序以自己的 bundle 身份发通知；短时子进程不创建 GUI。
/// 不经 shell，正文中的引号、换行和路径都原样传递；子进程日志不污染 CLI JSON。
pub fn notify(subtitle: &str, body: &str) -> anyhow::Result<()> {
    use anyhow::{bail, Context};

    if !crate::SettingsStore::open()?.load()?.notify.enabled {
        return Ok(());
    }
    let app = bundled_binary("yali-app")
        .context("原生通知需要完整 Yali.app，请使用 App 内的 CLI")?;
    let result = std::process::Command::new(app)
        .args(["--yali-notify", subtitle, body])
        .output()
        .context("启动鸭梨通知失败")?;
    if !result.status.success() {
        bail!("鸭梨通知失败：{}", String::from_utf8_lossy(&result.stderr).trim());
    }
    Ok(())
}

fn binary_in(contents: &Path, name: &str) -> Option<PathBuf> {
    let path = contents.join("MacOS").join(name);
    path.is_file().then_some(path)
}

/// 只给子进程设置环境，不改 GUI / CLI 的进程全局环境。
pub fn command_environment(program: &Path) -> Vec<(&'static str, String)> {
    // 保留 libvips 上游相对结构。仅包内可执行文件获得隔离环境。
    if let Some(root) = windows_root().or_else(|| program.ancestors().skip(1).find(|p| p.join("yali-runtime.json").is_file()).map(Path::to_path_buf)) {
        if program.starts_with(&root) {
            let system = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Windows"));
            let paths = [program.parent().unwrap().to_path_buf(), root.join("runtime/bin"), system.join("System32"), system.clone()];
            return vec![
                ("PATH", std::env::join_paths(paths).unwrap().to_string_lossy().into_owned()),
                ("VIPSHOME", root.join("runtime/vips").to_string_lossy().into_owned()),
                ("GIO_MODULE_DIR", root.join("runtime/empty-modules").to_string_lossy().into_owned()),
            ];
        }
    }
    let Some(contents) = contents_for(program) else {
        return Vec::new();
    };
    let path_separator = if cfg!(windows) { ";" } else { ":" };
    vec![
        (
            "VIPSHOME",
            contents
                .join("Resources/vips")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "PATH",
            format!("{}{path_separator}/usr/bin{path_separator}/bin", contents.join("MacOS").display()),
        ),
        (
            "HEIF_PLUGIN_PATH",
            contents
                .join("Resources/empty-modules")
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "GIO_MODULE_DIR",
            contents
                .join("Resources/empty-modules")
                .to_string_lossy()
                .into_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_manifest_validates_engine_bytes_architecture_and_required_tools() {
        use sha2::{Digest, Sha256};
        let dir = std::env::temp_dir().join(format!("yali-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut pe = vec![0u8; 128];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..70].copy_from_slice(b"PE\0\0\x64\x86");
        std::fs::write(dir.join("tool.exe"), &pe).unwrap();
        let names = ["vips", "vipsheader", "vipsthumbnail", "cjpeg", "jpegtran", "pngquant", "oxipng", "gifsicle", "cwebp", "gif2webp"];
        let tools: serde_json::Map<String, serde_json::Value> = names.into_iter()
            .map(|name| (name.into(), serde_json::json!({"path":"tool.exe"}))).collect();
        let mut manifest = serde_json::json!({
            "schemaVersion":1, "target":"x86_64-pc-windows-msvc",
            "tools":tools, "files":{"tool.exe":format!("{:x}", Sha256::digest(&pe))}
        });
        let write = |value: &serde_json::Value| std::fs::write(dir.join("yali-runtime.json"), serde_json::to_vec(value).unwrap()).unwrap();
        write(&manifest);
        assert!(read_manifest(&dir).is_ok());
        std::fs::write(dir.join("tool.exe"), b"corrupt").unwrap();
        assert!(read_manifest(&dir).err().unwrap().to_string().contains("校验失败"));
        std::fs::write(dir.join("tool.exe"), &pe).unwrap();
        manifest["tools"].as_object_mut().unwrap().remove("cjpeg");
        write(&manifest);
        assert!(read_manifest(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn windows_marker_is_distinct_from_macos_and_unsafe_paths_fail_closed() {
        let dir = std::env::temp_dir().join(format!("yali-zip-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cli = dir.join("yali.exe");
        std::fs::write(&cli, b"MZ").unwrap();
        assert!(zip_root_for(&cli).is_none());
        std::fs::write(dir.join("yali-runtime.json"), b"{}").unwrap();
        assert_eq!(zip_root_for(&cli), Some(dir.canonicalize().unwrap()));
        assert!(contents_for(&cli).is_none());
        assert!(read_manifest(&dir).is_err());
        for raw in ["../yali.exe", "C:/yali.exe", "C:yali.exe", "\\\\server\\yali.exe", "/yali.exe", "a/../yali.exe"] {
            assert!(relative_file(&dir, raw).is_err(), "{raw}");
        }
        assert!(relative_file(&dir, "yali.exe").is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bundle_detection_follows_cli_symlink_and_does_not_search_outside_bundle() {
        let dir = std::env::temp_dir().join(format!("yali-runtime-test-{}", std::process::id()));
        let contents = dir.join("Moved App.app/Contents");
        std::fs::create_dir_all(contents.join("MacOS")).unwrap();
        std::fs::create_dir_all(contents.join("Resources")).unwrap();
        let cli = contents.join("MacOS/yali");
        std::fs::write(&cli, b"test").unwrap();
        assert!(contents_for(&cli).is_none());
        std::fs::write(contents.join("Resources/yali-runtime.json"), b"{}").unwrap();
        assert_eq!(contents_for(&cli), Some(contents.canonicalize().unwrap()));
        assert!(binary_in(&contents, "pngquant").is_none());
        #[cfg(unix)]
        {
            let link = dir.join("yali");
            std::os::unix::fs::symlink(&cli, &link).unwrap();
            assert_eq!(contents_for(&link), contents_for(&cli));
            assert!(command_environment(&link)
                .iter()
                .any(|(k, v)| *k == "VIPSHOME" && v.ends_with("Resources/vips")));
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
