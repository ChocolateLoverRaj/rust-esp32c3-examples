use std::collections::HashMap;
use std::sync::LazyLock;

pub const HTML: &str = "text/html";
pub const JS: &str = "application/javascript";
pub const WASM: &str = "application/wasm";

pub const EXTENSION_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("html", HTML);
    m.insert("js", JS);
    m.insert("wasm", WASM);
    m
});
