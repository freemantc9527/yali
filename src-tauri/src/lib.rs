#[tauri::command]
fn core_version() -> String {
    yali_core::CORE_VERSION.to_string()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![core_version])
        .run(tauri::generate_context!())
        .expect("error while running yali");
}
