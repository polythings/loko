use std::io;

use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;

use crate::Console;

/// `console.log` and `console.error` of the JavaScript host (feature `js`, `wasm32-unknown-unknown`):
/// browsers, Workers, and Node.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsConsole;

impl JsConsole {
    fn call(method: &str, line: &str) -> io::Result<()> {
        let console = Reflect::get(&js_sys::global(), &"console".into())
            .map_err(|_| io::Error::other("the host has no console"))?;
        let f: Function = Reflect::get(&console, &method.into())
            .ok()
            .and_then(|f| f.dyn_into().ok())
            .ok_or_else(|| io::Error::other(format!("the console has no {method}")))?;
        f.call1(&console, &line.into()).map(drop).map_err(|e| io::Error::other(format!("console.{method}: {e:?}")))
    }
}

impl Console for JsConsole {
    fn print_line(&self, line: &str) -> io::Result<()> {
        Self::call("log", line)
    }

    fn print_error_line(&self, line: &str) -> io::Result<()> {
        Self::call("error", line)
    }
}
