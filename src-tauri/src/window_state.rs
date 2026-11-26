//! 主窗口几何状态独立于用户设置，避免设置页保存时覆盖运行中的窗口位置。

use std::{fs, io::Write, path::Path, path::PathBuf, sync::Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tauri::{Manager, PhysicalPosition, PhysicalRect, PhysicalSize, Window, WindowEvent};

const MAIN_WINDOW: &str = "main";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Geometry {
    // macOS 使用桌面逻辑坐标，其他平台使用物理坐标；允许主屏左侧/上方的负数。
    x: i32,
    y: i32,
    // 内容区逻辑像素，避免 Retina / 普通屏切换时尺寸翻倍。
    width: f64,
    height: f64,
}

impl Geometry {
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.width.is_finite()
                && self.height.is_finite()
                && self.width > 0.0
                && self.height > 0.0,
            "窗口尺寸必须是有限正数"
        );
        Ok(())
    }

    fn capture(window: &Window) -> Result<Option<Self>> {
        if window.is_minimized()? || window.is_maximized()? || window.is_fullscreen()? {
            return Ok(None);
        }
        let scale = window.scale_factor()?;
        let position = desktop_position(window.outer_position()?, scale);
        let size = window.inner_size()?.to_logical::<f64>(scale);
        let geometry = Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        };
        geometry.validate()?;
        Ok(Some(geometry))
    }

    fn fit(
        self,
        area: &PhysicalRect<i32, u32>,
        scale: f64,
        frame: PhysicalSize<u32>,
        minimum: (f64, f64),
    ) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        let available = PhysicalSize::new(
            area.size.width.saturating_sub(frame.width).max(1),
            area.size.height.saturating_sub(frame.height).max(1),
        );
        let width = (self.width.max(minimum.0) * scale)
            .round()
            .clamp(1.0, available.width as f64) as u32;
        let height = (self.height.max(minimum.1) * scale)
            .round()
            .clamp(1.0, available.height as f64) as u32;
        let x = (self.x as i64).clamp(
            area.position.x as i64,
            area.position.x as i64 + area.size.width.saturating_sub(width + frame.width) as i64,
        );
        let y = (self.y as i64).clamp(
            area.position.y as i64,
            area.position.y as i64 + area.size.height.saturating_sub(height + frame.height) as i64,
        );
        (
            PhysicalPosition::new(x as i32, y as i32),
            PhysicalSize::new(width, height),
        )
    }

    fn restore(self, window: &Window) -> Result<()> {
        let monitors = window.available_monitors()?;
        let monitor = monitors
            .iter()
            .find(|monitor| {
                let position = desktop_position(*monitor.position(), monitor.scale_factor());
                #[cfg(target_os = "macos")]
                let size = monitor.size().to_logical::<u32>(monitor.scale_factor());
                #[cfg(not(target_os = "macos"))]
                let size = *monitor.size();
                contains(
                    position,
                    PhysicalSize::new(size.width, size.height),
                    self.x,
                    self.y,
                )
            })
            .cloned()
            .or(window.primary_monitor()?)
            .or_else(|| monitors.first().cloned());
        let Some(monitor) = monitor else {
            return Ok(());
        };
        let scale = monitor.scale_factor();
        let inner = window.inner_size()?;
        let outer = window.outer_size()?;
        let ratio = scale / window.scale_factor()?;
        let frame = PhysicalSize::new(
            (outer.width.saturating_sub(inner.width) as f64 * ratio).round() as u32,
            (outer.height.saturating_sub(inner.height) as f64 * ratio).round() as u32,
        );
        let config = window
            .config()
            .app
            .windows
            .iter()
            .find(|config| config.label == MAIN_WINDOW);
        let minimum = config
            .map(|config| {
                (
                    config.min_width.unwrap_or(1.0),
                    config.min_height.unwrap_or(1.0),
                )
            })
            .unwrap_or((1.0, 1.0));
        #[cfg(target_os = "macos")]
        let physical = Self {
            x: (self.x as f64 * scale).round() as i32,
            y: (self.y as f64 * scale).round() as i32,
            ..self
        };
        #[cfg(not(target_os = "macos"))]
        let physical = self;
        let (position, size) = physical.fit(monitor.work_area(), scale, frame, minimum);
        // macOS setter 在移动前就按当前屏缩放换算，直接传逻辑坐标与尺寸才能正确跨屏。
        #[cfg(target_os = "macos")]
        {
            window.set_size(size.to_logical::<f64>(scale))?;
            window.set_position(position.to_logical::<f64>(scale))?;
        }
        #[cfg(not(target_os = "macos"))]
        {
            window.set_position(position)?;
            window.set_size(size)?;
        }
        Ok(())
    }
}

fn desktop_position(position: PhysicalPosition<i32>, scale: f64) -> PhysicalPosition<i32> {
    #[cfg(target_os = "macos")]
    {
        let logical = position.to_logical::<i32>(scale);
        PhysicalPosition::new(logical.x, logical.y)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = scale;
        position
    }
}

fn contains(position: PhysicalPosition<i32>, size: PhysicalSize<u32>, x: i32, y: i32) -> bool {
    x as i64 >= position.x as i64
        && (x as i64) < position.x as i64 + size.width as i64
        && y as i64 >= position.y as i64
        && (y as i64) < position.y as i64 + size.height as i64
}

fn load(path: &Path) -> Result<Option<Geometry>> {
    let raw = match fs::read(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let geometry: Geometry = serde_json::from_slice(&raw)?;
    geometry.validate()?;
    Ok(Some(geometry))
}

fn write(path: &Path, geometry: Geometry) -> Result<()> {
    geometry.validate()?;
    fs::create_dir_all(path.parent().context("窗口状态路径缺少父目录")?)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temporary = path.with_extension(format!("json.tmp-{}-{stamp}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(&geometry)?)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}

struct WindowState {
    path: Option<PathBuf>,
    geometry: Mutex<Option<Geometry>>,
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let Some(webview) = app.get_webview_window(MAIN_WINDOW) else {
        return Ok(());
    };
    let window = webview.as_ref().window();
    let path = match yali_core::data::root() {
        Ok(root) => Some(root.join("window-state.json")),
        Err(error) => {
            eprintln!("窗口状态路径不可用：{error:#}");
            None
        }
    };
    if let Some(path) = &path {
        match load(path).and_then(|geometry| geometry.map(|g| g.restore(&window)).transpose()) {
            Ok(_) => {}
            Err(error) => eprintln!("窗口状态恢复失败，使用默认窗口：{error:#}"),
        }
    }
    app.manage(WindowState {
        path,
        geometry: Mutex::new(None),
    });
    finish_setup(window)
}

fn finish_setup(window: Window) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        // Tao 的 set_size / set_position 将操作排入主队列；同队列后续捕获并显示。
        after_restore(dispatch2::DispatchQueue::main(), move || {
            if let Err(error) = remember_and_show(&window) {
                eprintln!("窗口显示失败：{error:#}");
            }
        });
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    remember_and_show(&window)
}

#[cfg(target_os = "macos")]
fn after_restore(queue: &dispatch2::DispatchQueue, finish: impl FnOnce() + Send + 'static) {
    queue.exec_async(finish);
}

fn remember_and_show(window: &Window) -> tauri::Result<()> {
    if let Some(state) = window.try_state::<WindowState>() {
        match Geometry::capture(window) {
            Ok(geometry) => *state.geometry.lock().unwrap_or_else(|e| e.into_inner()) = geometry,
            Err(error) => eprintln!("窗口状态读取失败：{error:#}"),
        }
    }
    window.show()
}

fn capture(window: &Window) {
    if window.label() != MAIN_WINDOW || !window.is_visible().unwrap_or(false) {
        return;
    }
    if let Some(state) = window.try_state::<WindowState>() {
        match Geometry::capture(window) {
            Ok(Some(geometry)) => {
                *state.geometry.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry)
            }
            Ok(None) => {}
            Err(error) => eprintln!("窗口状态读取失败：{error:#}"),
        }
    }
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW {
        return;
    }
    if matches!(
        event,
        WindowEvent::Moved(_)
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. }
            | WindowEvent::CloseRequested { .. }
    ) {
        capture(window);
    }
    if matches!(event, WindowEvent::CloseRequested { .. }) {
        save(window.app_handle());
    }
}

pub fn capture_main(app: &tauri::AppHandle) {
    if let Some(webview) = app.get_webview_window(MAIN_WINDOW) {
        capture(&webview.as_ref().window());
    }
}

pub fn save(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<WindowState>() else {
        return;
    };
    let geometry = *state.geometry.lock().unwrap_or_else(|e| e.into_inner());
    if let (Some(path), Some(geometry)) = (&state.path, geometry) {
        if let Err(error) = write(path, geometry) {
            eprintln!("窗口状态保存失败：{error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn first_show_is_deferred_until_queued_geometry_changes_complete() {
        use std::sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        };
        let queue = dispatch2::DispatchQueue::new("app.yali.window-state.test", None);
        let restored = Arc::new(AtomicBool::new(false));
        let changed = restored.clone();
        let (release, wait) = mpsc::channel();
        queue.exec_async(move || {
            wait.recv().unwrap();
            changed.store(true, Ordering::SeqCst);
        });
        let (shown, result) = mpsc::channel();
        after_restore(&queue, move || {
            shown.send(restored.load(Ordering::SeqCst)).unwrap();
        });
        release.send(()).unwrap();
        assert!(
            result
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            "首次显示必须等原生恢复任务执行后再开始"
        );
    }

    fn geometry() -> Geometry {
        Geometry {
            x: 200,
            y: 100,
            width: 980.0,
            height: 640.0,
        }
    }

    #[test]
    fn geometry_fits_work_area_after_display_disconnect() {
        let area = PhysicalRect {
            position: PhysicalPosition::new(0, 25),
            size: PhysicalSize::new(1440, 875),
        };
        let saved = Geometry {
            x: 3000,
            y: -900,
            width: 2000.0,
            height: 1200.0,
        };
        let (position, size) = saved.fit(&area, 1.0, PhysicalSize::new(0, 28), (640.0, 420.0));
        assert_eq!(position, PhysicalPosition::new(0, 25));
        assert_eq!(size, PhysicalSize::new(1440, 847));
    }

    #[test]
    fn negative_monitor_coordinates_and_retina_size_are_preserved() {
        let area = PhysicalRect {
            position: PhysicalPosition::new(-3000, 50),
            size: PhysicalSize::new(3000, 1950),
        };
        let saved = Geometry {
            x: -2500,
            ..geometry()
        };
        let (position, size) = saved.fit(&area, 2.0, PhysicalSize::new(0, 56), (640.0, 420.0));
        assert_eq!(position, PhysicalPosition::new(-2500, 100));
        assert_eq!(size, PhysicalSize::new(1960, 1280));
        assert!(contains(area.position, area.size, saved.x, saved.y));
        assert!(!contains(area.position, area.size, 0, saved.y));
        let (_, size) = saved.fit(&area, 1.0, PhysicalSize::new(0, 28), (640.0, 420.0));
        assert_eq!(size, PhysicalSize::new(980, 640));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_desktop_coordinates_do_not_depend_on_screen_scale() {
        assert_eq!(
            desktop_position(PhysicalPosition::new(-2400, 200), 2.0),
            desktop_position(PhysicalPosition::new(-1200, 100), 1.0),
        );
    }

    #[test]
    fn respects_configured_minimum_and_rejects_invalid_sizes() {
        let area = PhysicalRect {
            position: PhysicalPosition::new(0, 0),
            size: PhysicalSize::new(1920, 1080),
        };
        let saved = Geometry {
            width: 10.0,
            height: 10.0,
            ..geometry()
        };
        let (_, size) = saved.fit(&area, 1.0, PhysicalSize::new(0, 28), (640.0, 420.0));
        assert_eq!(size, PhysicalSize::new(640, 420));
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(Geometry {
                width,
                ..geometry()
            }
            .validate()
            .is_err());
        }
    }

    #[test]
    fn state_roundtrip_replaces_atomically_and_corruption_is_recoverable() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("yali-window-state-{}-{stamp}", std::process::id()));
        let path = dir.join("window-state.json");
        assert_eq!(load(&path).unwrap(), None);
        write(&path, geometry()).unwrap();
        assert_eq!(load(&path).unwrap(), Some(geometry()));
        let changed = Geometry {
            x: -1200,
            width: 1200.0,
            ..geometry()
        };
        write(&path, changed).unwrap();
        assert_eq!(load(&path).unwrap(), Some(changed));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::write(&path, b"{truncated").unwrap();
        assert!(load(&path).is_err());
        write(&path, geometry()).unwrap();
        assert_eq!(load(&path).unwrap(), Some(geometry()));
        fs::remove_dir_all(dir).unwrap();
    }
}
