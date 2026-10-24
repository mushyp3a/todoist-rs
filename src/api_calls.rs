use std::future::Future;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::Sender,
};

use color_eyre::eyre::{eyre, Context, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::{app::Msg, projects::Project, sections::Section, tasks::Task};

const API_URL: &str = "https://api.todoist.com/api/v1";

// 200 is the largest page the Unified API v1 accepts; larger values return an
// empty result set instead of an error.
const PAGE_LIMIT: u32 = 200;

static PENDING: AtomicUsize = AtomicUsize::new(0);

/// How many background requests are in flight.
pub fn pending() -> usize {
    PENDING.load(Ordering::Relaxed)
}

/// Runs an API call in the background and reports the outcome as a `Msg`.
///
/// `restore` is put back into the list if the call fails, undoing the
/// optimistic local change; `note` is shown when it succeeds.
pub fn spawn_op(
    tx: Sender<Msg>,
    restore: Option<Task>,
    note: Option<String>,
    call: impl Future<Output = Result<Option<Task>>> + Send + 'static,
) {
    PENDING.fetch_add(1, Ordering::Relaxed);
    tokio::spawn(async move {
        let outcome = call.await;
        PENDING.fetch_sub(1, Ordering::Relaxed);
        let _ = match outcome {
            Ok(task) => tx.send(Msg::Done {
                task: task.map(Box::new),
                note,
            }),
            Err(error) => tx.send(Msg::Failed {
                error: format!("{error:#}"),
                restore: restore.map(Box::new),
            }),
        };
    });
}

/// Turns a non-2xx response into an error that includes the response body.
async fn check(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let hint = match status {
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "\nThis is most likely an incorrect token. Check your config file."
        }
        _ => "",
    };
    Err(eyre!("Todoist API error {status}: {body}{hint}"))
}

async fn send(request: RequestBuilder) -> Result<String> {
    let response = request.send().await.context("Failed to reach Todoist")?;
    check(response)
        .await?
        .text()
        .await
        .context("Failed to read response")
}

fn next_cursor(page: &serde_json::Value) -> Option<String> {
    page.get("next_cursor")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

/// Fetches every page of a list endpoint such as `projects` or `tasks`.
async fn fetch_all<T: DeserializeOwned>(client: &Client, path: &str) -> Result<Vec<T>> {
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut request = client
            .get(format!("{API_URL}/{path}"))
            .query(&[("limit", PAGE_LIMIT)]);
        if let Some(cursor) = &cursor {
            request = request.query(&[("cursor", cursor)]);
        }

        let text = send(request)
            .await
            .wrap_err_with(|| format!("Failed to fetch {path}"))?;
        let mut page: serde_json::Value =
            serde_json::from_str(&text).wrap_err_with(|| format!("Failed to parse {path}"))?;

        cursor = next_cursor(&page);
        // Paginated responses wrap the list in `results`.
        let items = if page.get("results").is_some() {
            page["results"].take()
        } else {
            page
        };
        let items: Vec<T> = serde_json::from_value(items)
            .wrap_err_with(|| format!("Failed to deserialize {path}"))?;
        all.extend(items);

        if cursor.is_none() {
            return Ok(all);
        }
    }
}

pub async fn fetch_projects(client: &Client) -> Result<Vec<Project>> {
    fetch_all(client, "projects").await
}

pub async fn fetch_tasks(client: &Client) -> Result<Vec<Task>> {
    fetch_all(client, "tasks").await
}

pub async fn fetch_sections(client: &Client) -> Result<Vec<Section>> {
    fetch_all(client, "sections").await
}

async fn send_for_task(request: RequestBuilder) -> Result<Task> {
    let text = send(request).await?;
    serde_json::from_str(&text)
        .wrap_err_with(|| format!("Failed to parse task (response was: {text})"))
}

pub async fn create_task(client: &Client, body: serde_json::Value) -> Result<Task> {
    send_for_task(client.post(format!("{API_URL}/tasks")).json(&body)).await
}

pub async fn update_task(client: &Client, task_id: &str, body: serde_json::Value) -> Result<Task> {
    send_for_task(
        client
            .post(format!("{API_URL}/tasks/{task_id}"))
            .json(&body),
    )
    .await
}

pub async fn close_task(client: &Client, task_id: &str) -> Result<()> {
    send(client.post(format!("{API_URL}/tasks/{task_id}/close"))).await?;
    Ok(())
}

pub async fn reopen_task(client: &Client, task_id: &str) -> Result<()> {
    send(client.post(format!("{API_URL}/tasks/{task_id}/reopen"))).await?;
    Ok(())
}

pub async fn delete_task(client: &Client, task_id: &str) -> Result<()> {
    send(client.delete(format!("{API_URL}/tasks/{task_id}"))).await?;
    Ok(())
}

pub async fn fetch_task(client: &Client, task_id: &str) -> Result<Task> {
    send_for_task(client.get(format!("{API_URL}/tasks/{task_id}"))).await
}

/// Creates a task from Todoist's quick-add syntax: `buy milk tomorrow p1 #shopping @errands`.
pub async fn quick_add(client: &Client, text: &str) -> Result<Task> {
    send_for_task(
        client
            .post(format!("{API_URL}/tasks/quick"))
            .json(&serde_json::json!({ "text": text })),
    )
    .await
}

pub enum MoveTarget {
    Project(String),
    Parent(String),
}

pub async fn move_task(client: &Client, task_id: &str, target: MoveTarget) -> Result<Task> {
    let body = match target {
        MoveTarget::Project(id) => serde_json::json!({ "project_id": id }),
        MoveTarget::Parent(id) => serde_json::json!({ "parent_id": id }),
    };
    send_for_task(
        client
            .post(format!("{API_URL}/tasks/{task_id}/move"))
            .json(&body),
    )
    .await
}
