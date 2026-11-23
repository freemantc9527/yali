#![cfg(target_os = "macos")]

use std::process::Command;

#[test]
fn incomplete_notification_request_fails_without_starting_the_workbench() {
    let output = Command::new(env!("CARGO_BIN_EXE_yali-app"))
        .args(["--yali-notify", "图片压完了"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("通知模式需要副标题和正文"));
}

#[test]
fn unbundled_notification_request_fails_cleanly_before_calling_native_center() {
    let output = Command::new(env!("CARGO_BIN_EXE_yali-app"))
        .args([
            "--yali-notify",
            "图片压完了",
            "1 张省了 21 KB\n含\"引号\"和 \\ 路径",
        ])
        .output()
        .unwrap();
    // UserNotifications 在没有 bundle 身份的进程中可能抛 ObjC 异常；这里应正常报错退出。
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("原生通知需要从完整 Yali.app 运行"));
}
