use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_JSON_DEPTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum JsonRpcErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    UnsupportedProtocol = -32001,
    InvalidRunIdentity = -32002,
    SequenceViolation = -32003,
    CapabilityUnavailable = -32004,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProtocolError {
    pub code: JsonRpcErrorCode,
    pub reason: &'static str,
}

impl fmt::Display for AgentProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.reason, self.code as i32)
    }
}

impl std::error::Error for AgentProtocolError {}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolMessage {
    Request {
        id: Value,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    Response {
        id: Value,
        result: Option<Value>,
        error: Option<Value>,
    },
}

/// Stateful monotonic sequence guard for one client connection. Durable replay
/// remains the service/journal authority; this guard detects duplicate or
/// backwards live events before they reach a client reducer.
#[derive(Debug, Default)]
pub struct RunSequenceTracker {
    last_by_run: HashMap<(String, String), u64>,
}

impl RunSequenceTracker {
    pub fn observe(
        &mut self,
        chat_id: &str,
        run_id: &str,
        seq: u64,
    ) -> Result<(), AgentProtocolError> {
        let key = (chat_id.to_string(), run_id.to_string());
        if self.last_by_run.get(&key).is_some_and(|last| seq <= *last) {
            return Err(error(
                JsonRpcErrorCode::SequenceViolation,
                "run_sequence_not_monotonic",
            ));
        }
        self.last_by_run.insert(key, seq);
        Ok(())
    }
}

pub fn validate_message(value: Value) -> Result<ProtocolMessage, AgentProtocolError> {
    if json_depth(&value) > MAX_JSON_DEPTH {
        return Err(error(
            JsonRpcErrorCode::InvalidRequest,
            "json_nesting_limit",
        ));
    }
    let object = value
        .as_object()
        .ok_or_else(|| error(JsonRpcErrorCode::InvalidRequest, "message_must_be_object"))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(error(
            JsonRpcErrorCode::InvalidRequest,
            "jsonrpc_must_be_2_0",
        ));
    }

    if let Some(method) = object.get("method").and_then(Value::as_str) {
        if method.trim().is_empty() {
            return Err(error(
                JsonRpcErrorCode::InvalidRequest,
                "method_must_be_non_empty",
            ));
        }
        if !known_method(method) {
            return Err(error(JsonRpcErrorCode::MethodNotFound, "method_not_found"));
        }
        let params = object.get("params").cloned();
        if let Some(id) = object.get("id") {
            if !valid_id(id) {
                return Err(error(
                    JsonRpcErrorCode::InvalidRequest,
                    "request_id_invalid",
                ));
            }
            if !request_method(method) {
                return Err(error(
                    JsonRpcErrorCode::InvalidRequest,
                    "notification_method_cannot_have_id",
                ));
            }
            validate_initialize(method, params.as_ref())?;
            return Ok(ProtocolMessage::Request {
                id: id.clone(),
                method: method.to_string(),
                params,
            });
        }
        if !notification_method(method) {
            return Err(error(
                JsonRpcErrorCode::InvalidRequest,
                "request_method_requires_id",
            ));
        }
        validate_initialize(method, params.as_ref())?;
        validate_run_event(method, params.as_ref())?;
        return Ok(ProtocolMessage::Notification {
            method: method.to_string(),
            params,
        });
    }

    let id = object
        .get("id")
        .filter(|id| valid_id(id))
        .ok_or_else(|| error(JsonRpcErrorCode::InvalidRequest, "response_id_invalid"))?
        .clone();
    let result = object.get("result").cloned();
    let error_value = object.get("error").cloned();
    if result.is_some() == error_value.is_some() {
        return Err(error(
            JsonRpcErrorCode::InvalidRequest,
            "response_requires_one_outcome",
        ));
    }
    if let Some(error_value) = &error_value {
        validate_response_error(error_value)?;
    }
    Ok(ProtocolMessage::Response {
        id,
        result,
        error: error_value,
    })
}

fn request_method(method: &str) -> bool {
    matches!(
        method,
        "initialize"
            | "workspace/status"
            | "config/get"
            | "config/update"
            | "models/list"
            | "agents/list"
            | "sessions/list"
            | "sessions/get"
            | "sessions/create"
            | "run/start"
            | "run/steer"
            | "run/cancel"
            | "run/replay"
            | "clarification/respond"
            | "context/compact"
            | "checkpoints/list"
            | "checkpoints/restore"
            | "shutdown"
    )
}

fn notification_method(method: &str) -> bool {
    matches!(
        method,
        "run/event" | "workspace/changed" | "host/log" | "host/status"
    )
}

fn known_method(method: &str) -> bool {
    request_method(method) || notification_method(method)
}

fn validate_initialize(method: &str, params: Option<&Value>) -> Result<(), AgentProtocolError> {
    if method != "initialize" {
        return Ok(());
    }
    let params = params
        .and_then(Value::as_object)
        .ok_or_else(|| error(JsonRpcErrorCode::InvalidParams, "initialize_params_invalid"))?;
    let min = params.get("protocol_min").and_then(Value::as_u64);
    let max = params.get("protocol_max").and_then(Value::as_u64);
    let (Some(min), Some(max)) = (min, max) else {
        return Err(error(
            JsonRpcErrorCode::InvalidParams,
            "initialize_version_range_invalid",
        ));
    };
    if min > max {
        return Err(error(
            JsonRpcErrorCode::InvalidParams,
            "initialize_version_range_invalid",
        ));
    }
    if min > u64::from(PROTOCOL_VERSION) || max < u64::from(PROTOCOL_VERSION) {
        return Err(error(
            JsonRpcErrorCode::UnsupportedProtocol,
            "unsupported_protocol",
        ));
    }
    Ok(())
}

fn validate_run_event(method: &str, params: Option<&Value>) -> Result<(), AgentProtocolError> {
    if method != "run/event" {
        return Ok(());
    }
    let params = params.and_then(Value::as_object).ok_or_else(|| {
        error(
            JsonRpcErrorCode::InvalidRunIdentity,
            "run_event_params_invalid",
        )
    })?;
    for field in ["chat_id", "run_id"] {
        if params
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(error(
                JsonRpcErrorCode::InvalidRunIdentity,
                "run_identity_invalid",
            ));
        }
    }
    if params.get("seq").and_then(Value::as_u64).unwrap_or(0) == 0 {
        return Err(error(
            JsonRpcErrorCode::SequenceViolation,
            "run_sequence_invalid",
        ));
    }
    if !params.get("event").is_some_and(Value::is_object) {
        return Err(error(
            JsonRpcErrorCode::InvalidParams,
            "run_event_payload_invalid",
        ));
    }
    Ok(())
}

fn validate_response_error(value: &Value) -> Result<(), AgentProtocolError> {
    let error_value = value
        .as_object()
        .ok_or_else(|| error(JsonRpcErrorCode::InvalidRequest, "response_error_invalid"))?;
    if !error_value.get("code").is_some_and(Value::is_number)
        || error_value
            .get("message")
            .and_then(Value::as_str)
            .is_none_or(|message| message.trim().is_empty())
    {
        return Err(error(
            JsonRpcErrorCode::InvalidRequest,
            "response_error_shape_invalid",
        ));
    }
    Ok(())
}

fn valid_id(value: &Value) -> bool {
    matches!(value, Value::String(value) if !value.trim().is_empty())
        || matches!(value, Value::Number(_))
}

fn json_depth(value: &Value) -> usize {
    match value {
        Value::Array(values) => 1 + values.iter().map(json_depth).max().unwrap_or(0),
        Value::Object(values) => 1 + values.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn error(code: JsonRpcErrorCode, reason: &'static str) -> AgentProtocolError {
    AgentProtocolError { code, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_run_identity_and_sequence() {
        let good = json!({"jsonrpc":"2.0","method":"run/event","params":{"chat_id":"chat-1","run_id":"run-1","seq":1,"event":{"type":"thinking"}}});
        assert!(validate_message(good).is_ok());
        let bad = json!({"jsonrpc":"2.0","method":"run/event","params":{"chat_id":"","run_id":"run-1","seq":0,"event":{}}});
        assert_eq!(
            validate_message(bad).unwrap_err().code,
            JsonRpcErrorCode::InvalidRunIdentity
        );
    }

    #[test]
    fn rejects_missing_ids_and_deep_json() {
        assert_eq!(
            validate_message(json!({"jsonrpc":"2.0","method":"initialize","id":null}))
                .unwrap_err()
                .code,
            JsonRpcErrorCode::InvalidRequest
        );
        let mut deep = json!(null);
        for _ in 0..MAX_JSON_DEPTH {
            deep = json!([deep]);
        }
        assert_eq!(
            validate_message(deep).unwrap_err().code,
            JsonRpcErrorCode::InvalidRequest
        );
    }

    #[test]
    fn rejects_incompatible_versions_and_non_monotonic_sequences() {
        assert_eq!(
            validate_message(json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocol_min":2,"protocol_max":2}}))
                .unwrap_err()
                .code,
            JsonRpcErrorCode::UnsupportedProtocol
        );
        let mut tracker = RunSequenceTracker::default();
        tracker.observe("chat", "run", 1).unwrap();
        assert_eq!(
            tracker.observe("chat", "run", 1).unwrap_err().code,
            JsonRpcErrorCode::SequenceViolation
        );
        assert_eq!(
            validate_message(json!({"jsonrpc":"2.0","id":"future","method":"future/method"}))
                .unwrap_err()
                .code,
            JsonRpcErrorCode::MethodNotFound
        );
    }

    #[test]
    fn rejects_method_direction_and_malformed_response_error() {
        assert_eq!(
            validate_message(
                json!({"jsonrpc":"2.0","id":"event","method":"run/event","params":{}})
            )
            .unwrap_err()
            .reason,
            "notification_method_cannot_have_id"
        );
        assert_eq!(
            validate_message(json!({"jsonrpc":"2.0","method":"run/start"}))
                .unwrap_err()
                .reason,
            "request_method_requires_id"
        );
        assert_eq!(
            validate_message(json!({"jsonrpc":"2.0","id":"request","error":{"code":-32001}}))
                .unwrap_err()
                .reason,
            "response_error_shape_invalid"
        );
    }
}
