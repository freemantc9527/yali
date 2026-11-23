//! App 主程序的短时通知模式：保留 app.yali 身份，不启动 Tauri / 工作台。
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{NSBundle, NSDate, NSError, NSRunLoop, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest,
    UNUserNotificationCenter,
};

pub fn run_if_requested() -> Option<Result<()>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().and_then(|arg| arg.to_str()) != Some("--yali-notify") {
        return None;
    }
    Some(match args.as_slice() {
        [_, subtitle, body] => match (subtitle.to_str(), body.to_str()) {
            (Some(subtitle), Some(body)) => send(subtitle, body),
            _ => Err(anyhow::anyhow!("通知内容必须为 UTF-8 文本")),
        },
        _ => Err(anyhow::anyhow!("通知模式需要副标题和正文")),
    })
}

fn send(subtitle: &str, body: &str) -> Result<()> {
    // UserNotifications 要求真实 App bundle，开发态裸二进制不冒充应用身份。
    let bundle = NSBundle::mainBundle();
    if bundle
        .bundleIdentifier()
        .as_deref()
        .map(NSString::to_string)
        .as_deref()
        != Some("app.yali")
    {
        bail!("原生通知需要从完整 Yali.app 运行");
    }
    let mtm = MainThreadMarker::new().context("通知模式必须运行在主线程")?;
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);

    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (tx, rx) = mpsc::channel();
    let authorization = RcBlock::new(move |granted: Bool, error: *mut NSError| {
        let result = native_result(error).and_then(|()| {
            if granted.as_bool() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "通知未获允许，可在系统设置 → 通知 → Yali 中开启"
                ))
            }
        });
        let _ = tx.send(result);
    });
    center.requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert,
        &authorization,
    );
    wait(rx)?;

    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str("鸭梨"));
    content.setSubtitle(&NSString::from_str(subtitle));
    content.setBody(&NSString::from_str(body));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let identifier = NSString::from_str(&format!("yali-{}-{stamp}", std::process::id()));
    let request =
        UNNotificationRequest::requestWithIdentifier_content_trigger(&identifier, &content, None);
    let (tx, rx) = mpsc::channel();
    let completion = RcBlock::new(move |error: *mut NSError| {
        let _ = tx.send(native_result(error));
    });
    center.addNotificationRequest_withCompletionHandler(&request, Some(&completion));
    wait(rx)
}

fn native_result(error: *mut NSError) -> Result<()> {
    // SAFETY: 系统在 completion block 执行期间保证非空 NSError 存活；不跨回调保留指针。
    match unsafe { error.as_ref() } {
        Some(error) => Err(anyhow::anyhow!("{}", error.localizedDescription())),
        None => Ok(()),
    }
}

fn wait(rx: Receiver<Result<()>>) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match rx.try_recv() {
            Ok(result) => return result,
            Err(TryRecvError::Disconnected) => bail!("系统通知请求中断"),
            Err(TryRecvError::Empty) => {}
        }
        if Instant::now() >= deadline {
            bail!("等待系统通知响应超时");
        }
        // 授权与投递期间继续泵主线程 run loop；不阻塞系统授权 UI。
        NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.02));
    }
}
