#![cfg(target_os = "macos")]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn notify_uses_own_app_and_keeps_helper_output_out_of_cli_stdout() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yali-notification-sender-{}-{stamp}",
        std::process::id()
    ));
    let contents = root.join("App With Spaces.app/Contents");
    fs::create_dir_all(contents.join("MacOS")).unwrap();
    fs::create_dir_all(contents.join("Resources")).unwrap();
    fs::write(contents.join("Resources/yali-runtime.json"), "{}").unwrap();
    let cli = contents.join("MacOS/yali");
    fs::copy(env!("CARGO_BIN_EXE_yali"), &cli).unwrap();
    let helper = contents.join("MacOS/yali-app");
    fs::write(&helper, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.receipt\"\nprintf 'helper output must stay captured\\n'\n").unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();

    // 缺失输入仍应汇总一条失败通知，不依赖任何真实图片引擎或系统通知权限。
    let output = Command::new(cli)
        .args(["compress", "missing.jpg", "--quiet", "--notify"])
        .current_dir(&root)
        .env("YALI_HOME", root.join("data"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let receipt = PathBuf::from(format!("{}.receipt", helper.display()));
    assert_eq!(
        fs::read_to_string(receipt).unwrap(),
        "--yali-notify\n图片压完了\n1 张失败\n"
    );
    assert!(
        output.stdout.is_empty(),
        "notification helper contaminated CLI stdout: {:?}",
        output.stdout
    );
}
