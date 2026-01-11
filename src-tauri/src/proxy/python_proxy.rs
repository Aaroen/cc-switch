use once_cell::sync::Lazy;
use regex::Regex;

const DEFAULT_PYTHON_PROXY_BASE: &str = "http://127.0.0.1:15722";

pub(crate) fn python_proxy_base() -> String {
    std::env::var("CC_SWITCH_PYTHON_PROXY_BASE")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_PYTHON_PROXY_BASE.to_string())
}

pub(crate) fn python_proxy_label() -> String {
    let base = python_proxy_base();
    match port_from_base(&base) {
        Some(port) => format!("Python代理({port})"),
        None => "Python代理".to_string(),
    }
}

fn port_from_base(base: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)://[^/]+:(\d+)$").expect("regex"));

    let base = base.trim().trim_end_matches('/');
    if let Some(c) = RE.captures(base) {
        return c.get(1).map(|m| m.as_str().to_string());
    }
    // 兜底：无 scheme 的情况
    if let Some((_, port)) = base.rsplit_once(':') {
        if port.chars().all(|c| c.is_ascii_digit()) {
            return Some(port.to_string());
        }
    }
    None
}

