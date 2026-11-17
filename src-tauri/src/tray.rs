//! T-015 菜单栏托盘：常驻入口 + 关窗收进菜单栏（不退出进程）。
//!
//! 托盘是 M2 各入口（剪贴板 / watch）的挂载点：开关后续会进托盘菜单，
//! 本卡先落「打开鸭梨 / 退出鸭梨」两项，保证关窗后进程仍在。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "yali-tray";
pub const ITEM_OPEN: &str = "yali.tray.open";
pub const ITEM_QUIT: &str = "yali.tray.quit";

/// 关窗行为：托盘开着就收进菜单栏，否则按普通 app 退出
#[derive(Debug, PartialEq, Eq)]
pub enum CloseAction {
    Hide,
    Quit,
}

pub fn close_action(tray_enabled: bool) -> CloseAction {
    if tray_enabled {
        CloseAction::Hide
    } else {
        CloseAction::Quit
    }
}

/// 按设置开/关托盘（已存在就只切可见性，不重建）
pub fn apply(app: &AppHandle, enabled: bool) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        return tray.set_visible(enabled);
    }
    if !enabled {
        return Ok(());
    }
    build(app)
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, ITEM_OPEN, "打开鸭梨", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ITEM_QUIT, "退出鸭梨", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("鸭梨 · 让图片没有压力")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            ITEM_OPEN => show_main_window(app),
            ITEM_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// 显示并聚焦主窗口（托盘点击 / 菜单项共用）
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_hides_when_tray_enabled() {
        assert_eq!(close_action(true), CloseAction::Hide);
    }

    #[test]
    fn close_quits_when_tray_disabled() {
        assert_eq!(close_action(false), CloseAction::Quit);
    }

    #[test]
    fn menu_ids_are_prefixed() {
        // 托盘菜单 id 与 webview 事件名分区（yali.tray.*），避免与命令/事件撞名
        for id in [ITEM_OPEN, ITEM_QUIT, TRAY_ID] {
            assert!(id.starts_with("yali"), "id 应以 yali 开头: {id}");
        }
    }
}