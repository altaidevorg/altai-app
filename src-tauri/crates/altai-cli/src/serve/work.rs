use altai_core::{
    CreateWorkInput, WorkInboxRecord, WorkItemRecord, WorkListFilter, WorkState, WorkStore,
    WorkStoreError, WorkspacePaths,
};
use serde_json::{json, Map, Value};

pub(super) const CAPABILITIES: [&str; 8] = [
    "work/list",
    "work/get",
    "work/create",
    "work/transition",
    "work/start",
    "work/ready-for-review",
    "work/review",
    "work/inbox/list",
];

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
        }
    }
}

pub(super) fn handles(method: &str) -> bool {
    CAPABILITIES.contains(&method)
}

/// Route canonical Work RPC methods against the workspace selected when
/// `altai-cli serve` started. Callers never provide a workspace path.
pub(super) fn dispatch(
    workspace: &WorkspacePaths,
    method: &str,
    params: Option<Value>,
) -> Result<Value, RpcError> {
    let params = object_params(params)?;
    let (project_id, store) = open_store(workspace)?;

    match method {
        "work/list" => {
            let filter = parse_filter(optional_string(&params, "filter")?)?;
            store
                .list_work(&project_id, filter)
                .map(|items| Value::Array(items.into_iter().map(work_item_value).collect()))
                .map_err(store_error)
        }
        "work/inbox/list" => store
            .list_work_inbox(&project_id)
            .map(|items| Value::Array(items.into_iter().map(work_inbox_value).collect()))
            .map_err(store_error),
        "work/get" => {
            let work_id = required_string(&params, "workId")?;
            store
                .get_work(work_id)
                .map(|item| item.map(work_item_value).unwrap_or(Value::Null))
                .map_err(store_error)
        }
        "work/create" => {
            let title = required_string(&params, "title")?.to_string();
            let description = optional_string(&params, "description")?
                .unwrap_or_default()
                .to_string();
            let acceptance_criteria = optional_string(&params, "acceptanceCriteria")?
                .unwrap_or_default()
                .to_string();
            let assignee_ref = optional_string(&params, "assigneeRef")?.map(str::to_string);
            store
                .create_work(CreateWorkInput {
                    project_id,
                    title,
                    description,
                    acceptance_criteria,
                    assignee_ref,
                })
                .map(work_item_value)
                .map_err(store_error)
        }
        "work/transition" => {
            let work_id = required_string(&params, "workId")?;
            let expected_revision = required_revision(&params)?;
            let next_state = required_string(&params, "nextState")?;
            let next_state = WorkState::parse(next_state)
                .ok_or_else(|| RpcError::invalid_params("invalid_nextState"))?;
            store
                .transition(work_id, expected_revision, next_state)
                .map(work_item_value)
                .map_err(store_error)
        }
        "work/start" => {
            let work_id = required_string(&params, "workId")?;
            let expected_revision = required_revision(&params)?;
            store
                .start_attempt(work_id, expected_revision)
                .map(work_item_value)
                .map_err(store_error)
        }
        "work/ready-for-review" => {
            let work_id = required_string(&params, "workId")?;
            let expected_revision = required_revision(&params)?;
            store
                .mark_attempt_ready_for_review(work_id, expected_revision)
                .map(work_item_value)
                .map_err(store_error)
        }
        "work/review" => {
            let work_id = required_string(&params, "workId")?;
            let expected_revision = required_revision(&params)?;
            let accept = params
                .get("accept")
                .and_then(Value::as_bool)
                .ok_or_else(|| RpcError::invalid_params("invalid_accept"))?;
            let guidance = optional_string(&params, "guidance")?.unwrap_or_default();
            if !accept && guidance.trim().is_empty() {
                return Err(RpcError::invalid_params("return_guidance_required"));
            }
            store
                .human_review(work_id, expected_revision, accept, guidance)
                .map(work_item_value)
                .map_err(store_error)
        }
        _ => Err(RpcError::internal("unsupported_work_method")),
    }
}

fn open_store(workspace: &WorkspacePaths) -> Result<(String, WorkStore), RpcError> {
    let project_id = workspace
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string();
    let store = WorkStore::open(&workspace.work_db()).map_err(store_error)?;
    store
        .ensure_project(&project_id, &project_id, &workspace.root.to_string_lossy())
        .map_err(store_error)?;
    Ok((project_id, store))
}

fn object_params(params: Option<Value>) -> Result<Map<String, Value>, RpcError> {
    match params {
        None | Some(Value::Null) => Ok(Map::new()),
        Some(Value::Object(params)) => Ok(params),
        Some(_) => Err(RpcError::invalid_params("invalid_work_params")),
    }
}

fn required_string<'a>(params: &'a Map<String, Value>, key: &str) -> Result<&'a str, RpcError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError::invalid_params(format!("invalid_{key}")))
}

fn optional_string<'a>(
    params: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, RpcError> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(RpcError::invalid_params(format!("invalid_{key}"))),
    }
}

fn required_revision(params: &Map<String, Value>) -> Result<i64, RpcError> {
    params
        .get("expectedRevision")
        .and_then(Value::as_i64)
        .filter(|revision| *revision > 0)
        .ok_or_else(|| RpcError::invalid_params("invalid_expectedRevision"))
}

fn parse_filter(filter: Option<&str>) -> Result<WorkListFilter, RpcError> {
    match filter.unwrap_or("my_active") {
        "my_active" | "my-active" | "active" => Ok(WorkListFilter::MyActive),
        "review" => Ok(WorkListFilter::Review),
        "backlog" => Ok(WorkListFilter::Backlog),
        "done" => Ok(WorkListFilter::Done),
        _ => Err(RpcError::invalid_params("invalid_filter")),
    }
}

fn store_error(error: WorkStoreError) -> RpcError {
    match error {
        WorkStoreError::InvalidState(_) => RpcError::invalid_params(error.to_string()),
        WorkStoreError::NotFound(_) => RpcError {
            code: -32002,
            message: error.to_string(),
        },
        _ => RpcError::internal(error.to_string()),
    }
}

fn work_item_value(item: WorkItemRecord) -> Value {
    json!({
        "id": item.id,
        "projectId": item.project_id,
        "title": item.title,
        "description": item.description,
        "acceptanceCriteria": item.acceptance_criteria,
        "state": item.state.as_str(),
        "assigneeRef": item.assignee_ref,
        "blocker": item.blocker,
        "revision": item.revision,
        "createdAtMs": item.created_at_ms,
        "updatedAtMs": item.updated_at_ms,
    })
}

fn work_inbox_value(item: WorkInboxRecord) -> Value {
    json!({
        "id": item.id,
        "workId": item.work_id,
        "kind": item.kind.as_str(),
        "title": item.title,
        "why": item.why,
        "createdAtMs": item.created_at_ms,
        "attemptId": item.attempt_id,
        "chatId": item.chat_id,
        "runId": item.run_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{dispatch, handles, CAPABILITIES};
    use altai_core::resolve_workspace_from;
    use serde_json::{json, Value};
    use std::path::Path;

    #[test]
    fn canonical_router_runs_the_complete_human_review_lifecycle() {
        let temporary = tempfile::tempdir().expect("temporary workspace");
        let workspace = resolve_workspace_from(Some(temporary.path()), Path::new("/unused"))
            .expect("resolved workspace");

        let created = dispatch(
            &workspace,
            "work/create",
            Some(json!({
                "title": "Ship canonical Work RPC",
                "description": "Use the workspace selected by serve",
                "acceptanceCriteria": "Accept and Return remain human decisions",
                "assigneeRef": "agent:altai"
            })),
        )
        .expect("create Work");
        assert_eq!(created["state"], "backlog");
        assert_eq!(
            created["projectId"],
            temporary.path().file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(
            created["acceptanceCriteria"],
            "Accept and Return remain human decisions"
        );
        assert!(created.get("acceptance_criteria").is_none());

        let work_id = created["id"].as_str().expect("work id");
        let listed = dispatch(
            &workspace,
            "work/list",
            Some(json!({"filter": "backlog", "workspacePath": "/ignored"})),
        )
        .expect("list Work");
        assert!(listed.is_array());
        assert_eq!(listed[0]["id"], work_id);

        let fetched =
            dispatch(&workspace, "work/get", Some(json!({"workId": work_id}))).expect("get Work");
        assert_eq!(fetched, created);

        let ready = dispatch(
            &workspace,
            "work/transition",
            Some(json!({
                "workId": work_id,
                "expectedRevision": created["revision"],
                "nextState": "ready"
            })),
        )
        .expect("transition Work");
        let started = dispatch(
            &workspace,
            "work/start",
            Some(json!({
                "workId": work_id,
                "expectedRevision": ready["revision"]
            })),
        )
        .expect("start Work");
        let in_review = dispatch(
            &workspace,
            "work/ready-for-review",
            Some(json!({
                "workId": work_id,
                "expectedRevision": started["revision"]
            })),
        )
        .expect("mark Work ready for review");
        let missing_guidance = dispatch(
            &workspace,
            "work/review",
            Some(json!({
                "workId": work_id,
                "expectedRevision": in_review["revision"],
                "accept": false,
                "guidance": "  "
            })),
        )
        .expect_err("Return must include guidance");
        assert_eq!(missing_guidance.message, "return_guidance_required");
        let done = dispatch(
            &workspace,
            "work/review",
            Some(json!({
                "workId": work_id,
                "expectedRevision": in_review["revision"],
                "accept": true,
                "guidance": "Evidence accepted"
            })),
        )
        .expect("review Work");
        assert_eq!(done["state"], "done");
    }

    #[test]
    fn router_exposes_only_canonical_methods_and_rejects_snake_case_params() {
        assert_eq!(CAPABILITIES.len(), 8);
        assert!(handles("work/ready-for-review"));
        assert!(handles("work/inbox/list"));
        assert!(!handles("work/tasks/list"));

        let temporary = tempfile::tempdir().expect("temporary workspace");
        let workspace = resolve_workspace_from(Some(temporary.path()), Path::new("/unused"))
            .expect("resolved workspace");
        let error = dispatch(
            &workspace,
            "work/get",
            Some(json!({"work_id": "work_wrong_shape"})),
        )
        .expect_err("snake_case params must not satisfy canonical RPC");
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "invalid_workId");
        assert_eq!(
            dispatch(
                &workspace,
                "work/get",
                Some(json!({"workId": "work_missing"}))
            )
            .expect("missing Work is nullable"),
            Value::Null
        );
    }
}
