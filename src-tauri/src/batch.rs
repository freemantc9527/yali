//! GUI 批次只消费完整快照；旧 command 和新 command 共用执行器。
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, sync::Arc};
use yali_core::{
    engines::registry::Registry,
    pipeline::{compress_file, CompressReport},
    Profile,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub job_id: String,
    pub path: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub batch_id: String,
    pub items: Vec<Item>,
    pub profile: Profile,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub job_id: String,
    pub path: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<CompressReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub batch_id: String,
    pub outcomes: Vec<Outcome>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub batch_id: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

pub fn validate(request: &Request) -> Result<(), String> {
    request.profile.validate().map_err(crate::err)?;
    if request.batch_id.trim().is_empty() || request.items.is_empty() {
        return Err("批次 ID 与图片列表不能为空".into());
    }
    let mut ids = HashSet::new();
    for item in &request.items {
        if item.job_id.trim().is_empty()
            || !ids.insert(&item.job_id)
            || !Path::new(&item.path).is_absolute()
        {
            return Err("任务 ID 必须非空且唯一，图片必须使用绝对路径".into());
        }
    }
    Ok(())
}

pub async fn execute(
    request: Request,
    emit: Arc<dyn Fn(Outcome) + Send + Sync>,
) -> Result<Response, String> {
    validate(&request)?; // 在 warmup 与任何产物之前拒绝整个非法请求
    let registry = Arc::new(Registry::discover(&request.profile));
    registry.warmup().await;
    let semaphore = Arc::new(tokio::sync::Semaphore::new(crate::max_concurrency()));
    let mut handles = Vec::new();
    for item in request.items {
        let semaphore = semaphore.clone();
        let registry = registry.clone();
        let profile = request.profile.clone();
        let notify = emit.clone();
        let backup = item.clone();
        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.map_err(|e| e.to_string())?;
            notify(Outcome {
                job_id: item.job_id.clone(),
                path: item.path.clone(),
                status: "started",
                report: None,
                error: None,
            });
            let result = compress_file(Path::new(&item.path), &profile, &registry).await;
            let outcome = match result {
                Ok(report) => Outcome {
                    job_id: item.job_id,
                    path: item.path,
                    status: report.status,
                    report: Some(report),
                    error: None,
                },
                Err(e) => Outcome {
                    job_id: item.job_id,
                    path: item.path,
                    status: "failed",
                    report: None,
                    error: Some(format!("{e:#}")),
                },
            };
            notify(outcome.clone());
            Ok::<_, String>(outcome)
        });
        handles.push((backup, handle));
    }
    let mut outcomes = Vec::new();
    for (item, handle) in handles {
        let outcome = match handle.await {
            Ok(Ok(outcome)) => outcome,
            result => Outcome {
                job_id: item.job_id,
                path: item.path,
                status: "failed",
                report: None,
                error: Some(match result {
                    Ok(Err(e)) => e,
                    Err(e) => e.to_string(),
                    _ => unreachable!(),
                }),
            },
        };
        emit(outcome.clone());
        outcomes.push(outcome);
    }
    Ok(Response {
        batch_id: request.batch_id,
        outcomes,
    })
}

#[tauri::command]
pub async fn compress_batch(app: tauri::AppHandle, request: Request) -> Result<Response, String> {
    use tauri::Emitter;
    let batch_id = request.batch_id.clone();
    execute(
        request,
        Arc::new(move |outcome| {
            let _ = app.emit(
                "yali://batch-progress",
                Event {
                    batch_id: batch_id.clone(),
                    outcome,
                },
            );
        }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn validates_before_execution_and_preserves_partial_success() {
        let dir = std::env::temp_dir().join(format!("yali-batch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("good.jpg");
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../benchkit/samples/photo.jpg"),
            &input,
        )
        .unwrap();
        let profile: Profile =
            serde_json::from_str(include_str!("../../fixtures/default.json")).unwrap();
        let mut request = Request {
            batch_id: "batch-1".into(),
            profile,
            items: vec![
                Item {
                    job_id: "ok".into(),
                    path: input.to_string_lossy().into_owned(),
                },
                Item {
                    job_id: "bad".into(),
                    path: dir.join("missing.jpg").to_string_lossy().into_owned(),
                },
            ],
        };
        let response = execute(request.clone(), Arc::new(|_| {})).await.unwrap();
        assert_eq!(response.outcomes[0].status, "done");
        assert_eq!(response.outcomes[1].status, "failed");
        assert!(response.outcomes[0]
            .report
            .as_ref()
            .unwrap()
            .output
            .is_some());
        request.items[1].job_id = "ok".into();
        assert!(validate(&request).is_err());
        request.items[1].job_id = "bad".into();
        request.profile.quality.level = Some(0);
        assert!(validate(&request).is_err());
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["batchId"], "batch-1");
        assert_eq!(json["outcomes"][1]["jobId"], "bad");
        std::fs::remove_dir_all(dir).ok();
    }
}
