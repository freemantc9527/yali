//! 已构建 App 是运行时唯一来源；开发态保留 Homebrew 查找。
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

pub fn bundled_binary(name: &str) -> Option<PathBuf> {
    binary_in(&contents()?, name)
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
    let Some(contents) = contents_for(program) else {
        return Vec::new();
    };
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
            format!("{}:/usr/bin:/bin", contents.join("MacOS").display()),
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
