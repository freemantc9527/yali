fn main() {
    // 坑账本 #2：`tauri::generate_context!` 在**非 dev** 构建时会校验 frontendDist 路径存在，
    // 而 `ui/dist/` 是构建产物（不入库）。全新 clone 或 `cargo clean` 之后直接
    // `cargo build --release -p yali-app` 会 panic：
    //   "The `frontendDist` configuration is set to \"../ui/dist\" but this path doesn't exist"
    // 这里在 codegen 之前兜一个占位产物，保证「无前端产物也能编译」。
    // 正式产物仍由 `pnpm --dir ui build` 生成并覆盖（vite emptyOutDir）。
    ensure_frontend_dist_placeholder();
    // Embed one manifest in every linkable target, including the library test harness.
    // rustc-link-arg-tests only covers integration tests, not the library's unit tests.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        tauri_build::try_build(
            tauri_build::Attributes::new().windows_attributes(
                tauri_build::WindowsAttributes::new_without_app_manifest(),
            ),
        ).expect("Tauri Windows build configuration");
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    } else {
        tauri_build::build();
    }
}

fn ensure_frontend_dist_placeholder() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let dist = std::path::Path::new(&manifest_dir).join("../ui/dist");
    let index = dist.join("index.html");
    if index.exists() {
        return;
    }
    if let Err(err) = std::fs::create_dir_all(&dist) {
        println!("cargo:warning=无法创建 ui/dist 占位目录: {err}");
        return;
    }
    let placeholder = "<!doctype html><html lang=\"zh-Hans\"><head><meta charset=\"utf-8\">\
<title>Yali</title></head><body>前端产物缺失（请运行 pnpm --dir ui build）</body></html>";
    if let Err(err) = std::fs::write(&index, placeholder) {
        println!("cargo:warning=无法写入 ui/dist 占位页: {err}");
    }
}
