//! Finder 快速操作与稳定 CLI 入口。只管理本应用的两个 workflow。
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const INFO: &str =
    include_str!("../../../workflows/templates/quick-action.template/Contents/Info.plist");
const WORKFLOW: &str =
    include_str!("../../../workflows/templates/quick-action.template/Contents/document.wflow");
const TYPES: &[&str] = &[
    "public.jpeg",
    "public.png",
    "com.compuserve.gif",
    "org.webmproject.webp",
    "public.avif",
    "public.heic",
    "public.heif",
    "public.tiff",
    "com.microsoft.bmp",
    "public.jpeg-xl",
];
const ACTIONS: &[(&str, &str, &str)] = &[
    ("鸭梨 · 压一下", "app.yali.quick-action.compress", "default"),
    ("鸭梨 · 转 JPG", "app.yali.quick-action.to-jpg", "jpg"),
];
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub installed: bool,
    pub needs_repair: bool,
    pub can_install: bool,
    pub cli_path: String,
    pub message: String,
}

struct Locations {
    contents: Option<PathBuf>,
    root: PathBuf,
    home: PathBuf,
}

impl Locations {
    fn current() -> Result<Self> {
        Ok(Self {
            contents: crate::runtime::contents(),
            root: crate::data::root()?,
            home: PathBuf::from(std::env::var_os("HOME").context("无法确定用户主目录")?).canonicalize()?,
        })
    }

    fn cli_link(&self) -> PathBuf {
        self.root.join("bin/yali")
    }

    fn lock(&self) -> Result<fs::File> {
        fs::create_dir_all(&self.root)?;
        let file = fs::OpenOptions::new().read(true).write(true).create(true)
            .truncate(false).open(self.root.join(".services.lock"))?;
        file.lock()?;
        Ok(file)
    }
    fn services(&self) -> PathBuf {
        self.home.join("Library/Services")
    }
    fn destination(&self, title: &str) -> PathBuf {
        self.services().join(format!("{title}.workflow"))
    }

    fn cli(&self) -> Result<PathBuf> {
        let contents = self
            .contents
            .as_ref()
            .context("请使用构建后的 Yali.app 安装右键操作")?;
        let app = contents.parent().context("App 路径无效")?;
        let folder = app.parent().context("App 路径无效")?;
        if folder != Path::new("/Applications") && folder != self.home.join("Applications") {
            bail!("请先将鸭梨移入 /Applications 或 ~/Applications，再安装右键操作");
        }
        let cli = contents.join("MacOS/yali");
        if !cli.is_file() {
            bail!("App 缺少内嵌 CLI，请重新构建或安装完整应用");
        }
        Ok(cli)
    }

    fn documents(&self, title: &str, bid: &str, profile: &str) -> (String, String) {
        let types = TYPES
            .iter()
            .filter(|t| profile != "jpg" || **t != "public.jpeg")
            .map(|t| format!("<string>{t}</string>"))
            .collect::<Vec<_>>()
            .join("\n");
        let info = INFO
            .replace("__BID__", bid)
            .replace("__TITLE__", title)
            .replace("__SEND_TYPES__", &types);
        let cli = shell_quote(&self.cli_link().to_string_lossy());
        let script = format!(
            "if [ ! -x {cli} ]; then\n  /usr/bin/osascript -e 'display alert \"鸭梨无法运行\" message \"请重新打开鸭梨，或将它移入应用程序文件夹后安装右键操作。\"'\n  exit 1\nfi\nexec {cli} compress \"$@\" --profile {profile} --quiet --notify"
        );
        (info, WORKFLOW.replace("__SCRIPT__", &xml_escape(&script)))
    }

    fn status(&self) -> Status {
        let mut count = 0;
        let mut exact = true;
        for &(title, bid, profile) in ACTIONS {
            let dest = self.destination(title);
            if owned(&dest, bid) {
                count += 1;
                let (info, workflow) = self.documents(title, bid, profile);
                exact &= fs::read_to_string(dest.join("Contents/Info.plist"))
                    .ok()
                    .as_deref()
                    == Some(&info)
                    && fs::read_to_string(dest.join("Contents/document.wflow"))
                        .ok()
                        .as_deref()
                        == Some(&workflow);
            }
        }
        let cli = self.cli();
        let link_ok = cli
            .as_ref()
            .is_ok_and(|cli| fs::read_link(self.cli_link()).ok().as_ref() == Some(cli));
        let installed = count == ACTIONS.len() && exact && link_ok;
        Status {
            installed,
            needs_repair: count > 0 && !installed,
            can_install: cli.is_ok(),
            cli_path: self.cli_link().to_string_lossy().into_owned(),
            message: match cli {
                Err(e) => e.to_string(),
                Ok(_) if installed => "已安装两条操作；若菜单未显示，请在系统设置中启用。".into(),
                Ok(_) if count > 0 => "右键操作需要修复。".into(),
                Ok(_) => "尚未安装右键操作。".into(),
            },
        }
    }

    fn update_link(&self, cli: &Path) -> Result<()> {
        let link = self.cli_link();
        if fs::symlink_metadata(&link).is_ok() && !managed_link(&link) {
            bail!("{} 已存在且不属于鸭梨，未覆盖", link.display());
        }
        fs::create_dir_all(link.parent().unwrap())?;
        let temp = link.with_file_name(format!(
            ".yali-link-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::os::unix::fs::symlink(cli, &temp)?;
        if let Err(e) = fs::rename(&temp, &link) {
            let _ = fs::remove_file(&temp);
            return Err(e.into());
        }
        Ok(())
    }

    fn install(&self) -> Result<()> {
        let cli = self.cli()?;
        // 全部预检在写入之前；同名的第三方文件和符号链接不接管。
        for &(title, bid, _) in ACTIONS {
            let dest = self.destination(title);
            if fs::symlink_metadata(&dest).is_ok() && !owned(&dest, bid) {
                bail!("{} 已存在且不属于鸭梨，未覆盖", dest.display());
            }
        }
        fs::create_dir_all(self.services())?;
        let staging = self.services().join(format!(
            ".yali-install-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging)?;
        let mut backups = Vec::new();
        let mut published = Vec::new();
        let previous_link = fs::read_link(self.cli_link()).ok();
        let mut link_changed = false;
        let result = (|| -> Result<()> {
            for &(title, bid, profile) in ACTIONS {
                let stage = staging.join(title);
                fs::create_dir_all(stage.join("Contents"))?;
                let (info, workflow) = self.documents(title, bid, profile);
                fs::write(stage.join("Contents/Info.plist"), info)?;
                fs::write(stage.join("Contents/document.wflow"), workflow)?;
            }
            self.update_link(&cli)?;
            link_changed = true;
            for &(title, _, _) in ACTIONS {
                let dest = self.destination(title);
                if dest.exists() {
                    let backup = staging.join(format!("{title}.backup"));
                    fs::rename(&dest, &backup)?;
                    backups.push((backup, dest.clone()));
                }
                fs::rename(staging.join(title), &dest)?;
                published.push(dest);
            }
            Ok(())
        })();
        if result.is_err() {
            for dest in published.iter().rev() {
                fs::remove_dir_all(dest)?;
            }
            for (backup, dest) in backups.iter().rev() {
                fs::rename(backup, dest)?;
            }
            if link_changed {
                if let Some(previous) = previous_link { self.update_link(&previous)?; }
                else { fs::remove_file(self.cli_link())?; }
            }
        }
        // 只有本轮新建的暂存目录；回滚失败时保留备份供恢复。
        fs::remove_dir_all(&staging)?;
        result
    }

    fn uninstall(&self) -> Result<()> {
        for &(title, bid, _) in ACTIONS {
            let dest = self.destination(title);
            if owned(&dest, bid) {
                fs::remove_dir_all(dest)?;
            }
        }
        // 不移除 CLI 入口：终端脚本可继续使用；卸载右键不影响 CLI。
        Ok(())
    }
}

fn owned(path: &Path, bid: &str) -> bool {
    !path.is_symlink()
        && fs::read_to_string(path.join("Contents/Info.plist"))
            .is_ok_and(|s| s.contains(&format!("<string>{bid}</string>")))
}

fn managed_link(path: &Path) -> bool {
    fs::read_link(path).is_ok_and(|p| p.ends_with("Contents/MacOS/yali"))
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn status() -> Result<Status> {
    Ok(Locations::current()?.status())
}

pub fn install() -> Result<Status> {
    let locations = Locations::current()?;
    let _lock = locations.lock()?;
    locations.install()?;
    Ok(locations.status())
}

pub fn uninstall() -> Result<Status> {
    let locations = Locations::current()?;
    let _lock = locations.lock()?;
    locations.uninstall()?;
    Ok(locations.status())
}

/// App 移动后再打开，只修复已有托管入口，不自动安装用户 Services。
pub fn repair_existing_link() -> Result<()> {
    let locations = Locations::current()?;
    if managed_link(&locations.cli_link()) {
        let _lock = locations.lock()?;
        locations.update_link(&locations.cli()?)?;
    }
    Ok(())
}

pub fn refresh() {
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {
        fn NSUpdateDynamicServices();
    }
    // 本机 SDK NSApplication.h 声明的公开 Services 重扫描 API。
    unsafe { NSUpdateDynamicServices() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locations(dir: &Path) -> Locations {
        let home = dir.join("Home & O'Neil");
        let contents = home.join("Applications/Yali.app/Contents");
        fs::create_dir_all(contents.join("MacOS")).unwrap();
        fs::write(contents.join("MacOS/yali"), b"test").unwrap();
        Locations {
            contents: Some(contents),
            root: home.join(".yali"),
            home,
        }
    }

    #[test]
    fn install_repair_move_and_uninstall_preserve_user_data() {
        let dir = std::env::temp_dir().join(format!("yali-services-move-{}", std::process::id()));
        let mut loc = locations(&dir);
        assert!(!loc.status().installed);
        loc.install().unwrap();
        assert!(loc.status().installed);
        loc.install().unwrap();
        let jpg =
            fs::read_to_string(loc.destination(ACTIONS[1].0).join("Contents/Info.plist")).unwrap();
        assert!(!jpg.contains("<string>public.jpeg</string>"));
        let script = fs::read_to_string(
            loc.destination(ACTIONS[0].0)
                .join("Contents/document.wflow"),
        )
        .unwrap();
        assert!(script.contains("&amp; O'\\''Neil"));
        assert!(!script.contains("/opt/homebrew"));
        let old = loc.contents.take().unwrap();
        let new = old
            .parent()
            .unwrap()
            .with_file_name("Renamed.app")
            .join("Contents");
        fs::rename(old.parent().unwrap(), new.parent().unwrap()).unwrap();
        loc.contents = Some(new);
        assert!(loc.status().needs_repair);
        loc.update_link(&loc.cli().unwrap()).unwrap();
        assert!(loc.status().installed);
        fs::write(loc.root.join("settings.json"), b"keep").unwrap();
        loc.uninstall().unwrap();
        loc.uninstall().unwrap();
        assert!(!loc.status().installed);
        assert!(loc.root.join("settings.json").is_file());
        assert!(loc.cli_link().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn conflicting_workflow_and_regular_cli_are_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("yali-services-conflict-{}", std::process::id()));
        let loc = locations(&dir);
        fs::create_dir_all(loc.destination(ACTIONS[1].0)).unwrap();
        assert!(loc.install().is_err());
        assert!(!loc.destination(ACTIONS[0].0).exists());
        fs::remove_dir(loc.destination(ACTIONS[1].0)).unwrap();
        fs::create_dir_all(loc.cli_link().parent().unwrap()).unwrap();
        fs::write(loc.cli_link(), b"user script").unwrap();
        assert!(loc.install().is_err());
        assert_eq!(fs::read(loc.cli_link()).unwrap(), b"user script");
        loc.uninstall().unwrap();
        assert!(loc.cli_link().is_file());
        fs::remove_dir_all(dir).unwrap();
    }
}
