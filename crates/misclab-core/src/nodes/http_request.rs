//! HTTP 请求节点：GET/POST/HEAD/PUT/DELETE/… ，支持 HTTP/1.1 与 HTTP/2（ALPN 自动协商或强制），
//! 可设 User-Agent、Cookie、自定义请求头与请求体。基于 reqwest 阻塞客户端 + rustls。
//!
//! 注意：reqwest 阻塞客户端不能在已有 tokio 运行时里创建（Tauri 命令是异步的），故在一条
//! 全新的 OS 线程（`std::thread::scope`）里发请求，避开「运行时套运行时」panic。
use std::time::Duration;

use super::prelude::*;

#[allow(clippy::too_many_arguments)]
fn send(
    method: reqwest::Method,
    url: &str,
    version: &str,
    ua: &str,
    cookies: &str,
    headers: &str,
    content_type: &str,
    body: Vec<u8>,
    timeout: u64,
    follow: bool,
    insecure: bool,
) -> Result<(u16, String, String, Vec<u8>), String> {
    let mut b = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .redirect(if follow {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .danger_accept_invalid_certs(insecure);
    match version {
        "1.1" => b = b.http1_only(),
        "2.0" => b = b.http2_prior_knowledge(),
        _ => {}
    }
    if !ua.is_empty() {
        b = b.user_agent(ua);
    }
    let client = b.build().map_err(|e| format!("客户端构建失败: {e}"))?;

    let mut req = client.request(method.clone(), url);
    if !cookies.is_empty() {
        req = req.header(reqwest::header::COOKIE, cookies);
    }
    for line in headers.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            req = req.header(k.trim(), v.trim());
        }
    }
    if !content_type.is_empty() {
        req = req.header(reqwest::header::CONTENT_TYPE, content_type);
    }
    if !body.is_empty() && method != reqwest::Method::GET && method != reqwest::Method::HEAD {
        req = req.body(body);
    }

    let resp = req.send().map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status().as_u16();
    let ver = format!("{:?}", resp.version());
    let mut hdrs = String::new();
    for (k, v) in resp.headers() {
        hdrs.push_str(&format!("{}: {}\n", k, v.to_str().unwrap_or("<非文本>")));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| format!("读取响应失败: {e}"))?
        .to_vec();
    Ok((status, ver, hdrs, bytes))
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        _c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let url = pstr(p, "url", "").trim().to_string();
        if url.is_empty() {
            return Err(CoreError::Parse("请填写 URL。".into()));
        }
        let method_s = pstr(p, "method", "GET").to_ascii_uppercase();
        let method = reqwest::Method::from_bytes(method_s.as_bytes())
            .map_err(|_| CoreError::Parse(format!("非法请求方法: {method_s}")))?;
        let version = pstr(p, "httpVersion", "自动").to_string();
        let ua = pstr(p, "userAgent", "").to_string();
        let cookies = pstr(p, "cookies", "").to_string();
        let headers = pstr(p, "headers", "").to_string();
        let content_type = pstr(p, "contentType", "").to_string();
        let timeout = pnum(p, "timeout", 15.0).clamp(1.0, 300.0) as u64;
        let follow = pbool(p, "followRedirects", true);
        let insecure = pbool(p, "insecure", false);
        let body = match i.get("body") {
            Some(PortValue::None) | None => pstr(p, "body", "").as_bytes().to_vec(),
            Some(_) => in_bytes(i, "body")?,
        };

        // 在独立线程里发请求，避免嵌套 tokio 运行时。
        let m2 = method.clone();
        let result: Result<(u16, String, String, Vec<u8>), String> =
            std::thread::scope(|s| {
                match s
                    .spawn(|| {
                        send(
                            m2,
                            &url,
                            &version,
                            &ua,
                            &cookies,
                            &headers,
                            &content_type,
                            body,
                            timeout,
                            follow,
                            insecure,
                        )
                    })
                    .join()
                {
                    Ok(r) => r,
                    Err(_) => Err("请求线程崩溃".to_string()),
                }
            });

        let (status, ver, hdrs, bytes) = result.map_err(CoreError::Other)?;
        let mut m = PortMap::new();
        m.insert(
            "text".into(),
            PortValue::Text(String::from_utf8_lossy(&bytes).into_owned()),
        );
        m.insert(
            "bytes".into(),
            PortValue::Bytes(Arc::from(bytes.into_boxed_slice())),
        );
        m.insert("status".into(), PortValue::Number(status as f64));
        m.insert("headers".into(), PortValue::Text(hdrs));
        m.insert(
            "report".into(),
            PortValue::Text(format!("{ver} {status} · {method_s} {url}")),
        );
        Ok(m)
    }
}

pub fn register(reg: &mut NodeRegistry) {
    reg.register(
        desc(
            "http_request",
            UTIL,
            "HTTP 请求",
            INDIGO,
            vec![opt("body", "请求体(可选)", PortType::Any)],
            vec![
                req("text", "响应体", PortType::Text),
                opt("bytes", "响应字节", PortType::Bytes),
                opt("status", "状态码", PortType::Number),
                opt("headers", "响应头", PortType::Text),
                opt("report", "信息", PortType::Text),
            ],
            vec![
                ParamSpec::select("method", "方法", &["GET", "POST", "HEAD", "PUT", "DELETE", "PATCH", "OPTIONS"], "GET"),
                ParamSpec::text("url", "URL", "https://httpbin.org/get", false),
                ParamSpec::select("httpVersion", "HTTP 版本", &["自动", "1.1", "2.0"], "自动"),
                ParamSpec::text(
                    "userAgent",
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36",
                    false,
                ),
                ParamSpec::text("cookies", "Cookie", "", false),
                ParamSpec::text("headers", "请求头(每行 K: V)", "", true),
                ParamSpec::text("contentType", "Content-Type", "", false),
                ParamSpec::text("body", "请求体(无输入时用)", "", true),
                ParamSpec::number("timeout", "超时(秒)", 1.0, 300.0, 1.0, 15.0),
                ParamSpec::toggle("followRedirects", "跟随重定向", true),
                ParamSpec::toggle("insecure", "忽略证书错误", false),
            ],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::node::PortMap;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    // Offline-safe: an empty URL must error before any network access.
    #[test]
    fn empty_url_errors() {
        let reg = default_registry();
        let r = GraphExecutor::run_node(
            &reg,
            "http_request",
            &PortMap::new(),
            &serde_json::json!({ "url": "" }),
            &NullSink,
            &CancellationToken::new(),
        );
        assert!(r.is_err());
    }
}
