use std::collections::HashMap;
use std::fs;
use std::path::Path;

use boa_engine::property::Attribute;
use boa_engine::{Context, JsNativeError, JsResult, JsValue, NativeFunction, Source};
use boa_runtime::console::{Console, DefaultLogger};

use crate::fetch;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: node [options] [script.js] [arguments]");
        eprintln!("Options:");
        eprintln!("  -e, --eval <code>  Evaluate JavaScript code");
        eprintln!("  -p, --print <code> Evaluate and print result");
        eprintln!("  --version          Print version");
        return 1;
    }

    // Handle --version
    if args[0] == "--version" || args[0] == "-v" {
        println!("node v0.1.0 (boa-engine/wasm-sandbox)");
        return 0;
    }

    let mut context = Context::default();

    // Register console object globally (console.log, console.error, etc.)
    if let Err(e) = Console::register_with_logger(DefaultLogger, &mut context) {
        eprintln!("node: failed to register console: {e}");
        return 1;
    }

    // Register fetch() global function
    register_fetch(&mut context);

    match args[0].as_str() {
        "-e" | "--eval" => {
            if args.len() < 2 {
                eprintln!("node: -e requires an argument");
                return 1;
            }
            execute(&mut context, &args[1])
        }
        "-p" | "--print" => {
            if args.len() < 2 {
                eprintln!("node: -p requires an argument");
                return 1;
            }
            execute_and_print(&mut context, &args[1])
        }
        _ => {
            // Treat as a file path
            let file_path = &args[0];
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    let source =
                        Source::from_bytes(&content).with_path(Path::new(file_path));
                    match context.eval(source) {
                        Ok(_) => 0,
                        Err(err) => {
                            eprintln!("{err}");
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("node: cannot open '{}': {}", file_path, e);
                    1
                }
            }
        }
    }
}

fn register_fetch(context: &mut Context) {
    let fetch_fn = NativeFunction::from_fn_ptr(js_fetch);
    let callable: JsValue = fetch_fn.to_js_function(context.realm()).into();

    context
        .register_global_property(
            boa_engine::js_string!("fetch"),
            callable,
            Attribute::WRITABLE | Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
        )
        .expect("failed to register fetch");
}

/// JS `fetch(url, options?)` implementation.
///
/// Synchronous: calls the host bridge directly (no Promise wrapping needed
/// since Boa doesn't have a real async event loop in WASI p1).
///
/// Returns an object: `{ status, ok, headers, body, text() }`
fn js_fetch(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    // Parse url argument
    let url = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("fetch requires a URL argument"))?
        .to_string(context)?
        .to_std_string_escaped();

    // Parse optional options object
    let mut method = "GET".to_string();
    let mut headers = HashMap::new();
    let mut body: Option<String> = None;

    if let Some(opts) = args.get(1) {
        if opts.is_object() {
            let obj = opts.as_object().unwrap();

            // method
            if let Ok(m) = obj.get(boa_engine::js_string!("method"), context) {
                if !m.is_undefined() && !m.is_null() {
                    method = m.to_string(context)?.to_std_string_escaped();
                }
            }

            // headers
            if let Ok(h) = obj.get(boa_engine::js_string!("headers"), context) {
                if let Some(h_obj) = h.as_object() {
                    // Get keys via Object.keys equivalent
                    let keys = h_obj.own_property_keys(context)?;
                    for key in keys {
                        let key_str = key.to_string();
                        if let Ok(val) = h_obj.get(key, context) {
                            let val_str = val.to_string(context)?.to_std_string_escaped();
                            headers.insert(key_str, val_str);
                        }
                    }
                }
            }

            // body
            if let Ok(b) = obj.get(boa_engine::js_string!("body"), context) {
                if !b.is_undefined() && !b.is_null() {
                    body = Some(b.to_string(context)?.to_std_string_escaped());
                }
            }
        }
    }

    // Call the host fetch bridge
    let result = fetch::fetch(&url, &method, &headers, body.as_deref());

    match result {
        Ok(resp) => build_response_object(resp, context),
        Err(e) => Err(JsNativeError::typ()
            .with_message(format!("fetch failed: {e}"))
            .into()),
    }
}

fn build_response_object(
    resp: fetch::FetchResponse,
    context: &mut Context,
) -> JsResult<JsValue> {
    let obj = boa_engine::object::JsObject::with_null_proto();

    // status
    obj.set(
        boa_engine::js_string!("status"),
        JsValue::from(resp.status as i32),
        false,
        context,
    )?;

    // ok
    obj.set(
        boa_engine::js_string!("ok"),
        JsValue::from(resp.ok),
        false,
        context,
    )?;

    // body (as string)
    let body_str = boa_engine::JsString::from(resp.body.as_str());
    obj.set(
        boa_engine::js_string!("body"),
        JsValue::from(body_str.clone()),
        false,
        context,
    )?;

    // headers object
    let headers_obj = boa_engine::object::JsObject::with_null_proto();
    for (k, v) in &resp.headers {
        headers_obj.set(
            boa_engine::JsString::from(k.as_str()),
            JsValue::from(boa_engine::JsString::from(v.as_str())),
            false,
            context,
        )?;
    }
    obj.set(
        boa_engine::js_string!("headers"),
        JsValue::from(headers_obj),
        false,
        context,
    )?;

    // text() method - returns the body string
    let body_for_text = body_str;
    // SAFETY: The closure only captures a JsString which is reference-counted and thread-safe.
    let text_fn = unsafe {
        NativeFunction::from_closure(move |_, _, _ctx| {
            Ok(JsValue::from(body_for_text.clone()))
        })
    };
    obj.set(
        boa_engine::js_string!("text"),
        text_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // json() method - parses body as JSON via JSON.parse() (NOT eval, to prevent code injection)
    let body_for_json = boa_engine::JsString::from(resp.body.as_str());
    // SAFETY: The closure only captures a JsString which is reference-counted and thread-safe.
    let json_fn = unsafe {
        NativeFunction::from_closure(move |_, _, ctx| {
            let json_parse = ctx
                .eval(Source::from_bytes(b"JSON.parse"))
                .map_err(|_| JsNativeError::typ().with_message("JSON.parse not available"))?;
            let json_parse_fn = json_parse
                .as_callable()
                .ok_or_else(|| JsNativeError::typ().with_message("JSON.parse is not callable"))?;
            json_parse_fn
                .call(&JsValue::undefined(), &[JsValue::from(body_for_json.clone())], ctx)
                .map_err(|_| JsNativeError::syntax().with_message("invalid JSON").into())
        })
    };
    obj.set(
        boa_engine::js_string!("json"),
        json_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(JsValue::from(obj))
}

fn execute(context: &mut Context, code: &str) -> i32 {
    let source = Source::from_bytes(code);
    match context.eval(source) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn execute_and_print(context: &mut Context, code: &str) -> i32 {
    let source = Source::from_bytes(code);
    match context.eval(source) {
        Ok(val) => {
            match val.to_string(context) {
                Ok(output) => {
                    println!("{}", output.to_std_string_escaped());
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}
