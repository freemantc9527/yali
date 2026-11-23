// M-1 终验壳：证明 Tauri 栈在 CLT-only 工具链可编译。
// 真实 commands/tray/剪贴板 随 M1/M2 任务卡填充。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "macos")]
    if let Some(result) = yali_app_lib::notification::run_if_requested() {
        if let Err(e) = result {
            eprintln!("{e:#}");
            std::process::exit(1);
        }
        return;
    }
    yali_app_lib::run()
}
