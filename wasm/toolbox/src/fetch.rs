use std::collections::HashMap;

use serde_json::{Value, json};

// Host-provided functions for the fetch bridge.
// These are linked from the "sandbox" module by the Wasmtime host.
#[link(wasm_import_module = "sandbox")]
unsafe extern "C" {
    fn __sandbox_fetch(req_ptr: i32, req_len: i32) -> i32;
    fn __sandbox_fetch_response_len() -> i32;
    fn __sandbox_fetch_response_read(buf_ptr: i32, buf_len: i32) -> i32;
}

/// Response from a fetch call.
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub ok: bool,
}

/// Perform an HTTP fetch via the host sandbox bridge.
pub fn fetch(
    url: &str,
    method: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
) -> Result<FetchResponse, String> {
    let req = json!({
        "url": url,
        "method": method,
        "headers": headers,
        "body": body,
    });

    let req_bytes = serde_json::to_vec(&req).map_err(|e| format!("serialize error: {e}"))?;

    let result = unsafe { __sandbox_fetch(req_bytes.as_ptr() as i32, req_bytes.len() as i32) };

    if result == -1 {
        return Err("fetch bridge error: failed to communicate with host".into());
    }

    // Read response length
    let resp_len = unsafe { __sandbox_fetch_response_len() };
    if resp_len <= 0 {
        return Err("fetch bridge error: empty response".into());
    }

    // Read response bytes
    let mut resp_buf = vec![0u8; resp_len as usize];
    let read = unsafe { __sandbox_fetch_response_read(resp_buf.as_mut_ptr() as i32, resp_len) };
    if read < 0 {
        return Err("fetch bridge error: failed to read response".into());
    }
    resp_buf.truncate(read as usize);

    // Parse response JSON
    let resp: Value =
        serde_json::from_slice(&resp_buf).map_err(|e| format!("deserialize error: {e}"))?;

    // Check for error field
    if let Some(err) = resp.get("error").and_then(|v| v.as_str()) {
        if !err.is_empty() {
            return Err(err.to_string());
        }
    }

    let status = resp
        .get("status")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    let headers: HashMap<String, String> = resp
        .get("headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let body = resp
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);

    Ok(FetchResponse {
        status,
        headers,
        body,
        ok,
    })
}
