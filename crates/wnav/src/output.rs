//! Output helpers: single-line JSON emission (FR-4.1).

use serde::Serialize;

/// Print one compact single-line JSON document to stdout — the only machine
/// channel (SRS §2.4).
pub(crate) fn emit<T: Serialize>(v: &T) {
    println!(
        "{}",
        serde_json::to_string(v).expect("json serialize cannot fail here")
    );
}

/// Error JSON envelope: `{"ok":false,"error":{"code":...,"message":...,"hint":...}}`.
#[derive(Serialize)]
pub(crate) struct JsonErr {
    pub ok: bool,
    pub error: ErrorBody,
}

impl JsonErr {
    pub(crate) fn body(code: i32, message: &str, hint: Option<&str>) -> ErrorBody {
        ErrorBody {
            code,
            message: message.to_string(),
            hint: hint.map(str::to_string),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ErrorBody {
    pub code: i32,
    pub message: String,
    pub hint: Option<String>,
}
