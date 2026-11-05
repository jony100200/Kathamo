//! Talk to a host application from a phone or tablet controller.
//!
//! Kathamo apps on the desktop (Setu, Kolpona) expose their controls as
//! JSON-RPC, mostly as MCP (Model Context Protocol) tools. A Kathamo app on
//! Android can drive them with [`McpClient`]:
//!
//! ```no_run
//! # fn demo() -> Result<(), String> {
//! use kathamo::remote::{json, HttpEndpoint, McpClient};
//! let endpoint = HttpEndpoint::parse("http://100.64.0.2:3000/mcp", None)?;
//! let mut mcp = McpClient::new(endpoint);
//! mcp.initialize("my-phone-remote")?;
//! for tool in mcp.list_tools()? {
//!     println!("{} read-only={}", tool.name, tool.read_only);
//! }
//! let reply = mcp.call_tool("agent_list", json!({}))?;
//! println!("{}", kathamo::remote::tool_text(&reply));
//! # Ok(()) }
//! ```
//!
//! Calls block, so run them on a worker thread and call
//! [`UiWaker::wake`](crate::app::UiWaker::wake) when a reply lands.
//!
//! Only plain `http://` is spoken. Reach the host over a private network that
//! encrypts traffic (Tailscale, WireGuard) and protect the host with a bearer
//! token; never expose an unauthenticated control port to the internet.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub use serde_json::{json, Value};

/// One HTTP JSON endpoint on a host, with an optional bearer token.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpEndpoint {
    pub host: String,
    pub port: u16,
    /// Request path, starting with `/`.
    pub path: String,
    pub token: Option<String>,
    pub timeout: Duration,
}

impl HttpEndpoint {
    /// Parses `http://host[:port][/path]` (the scheme may be left out).
    /// `https://` is refused with a clear message because this client has no TLS.
    pub fn parse(url: &str, token: Option<&str>) -> Result<Self, String> {
        let url = url.trim();
        let rest = if let Some(rest) = url.strip_prefix("http://") {
            rest
        } else if url.starts_with("https://") {
            return Err(
                "https:// is not supported; use http:// over Tailscale or another private network"
                    .into(),
            );
        } else if url.contains("://") {
            return Err(format!("unsupported address: {url}"));
        } else {
            url
        };
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = match authority.rsplit_once(':') {
            Some((h, p)) if !p.contains(']') => {
                let port = p.parse::<u16>().map_err(|_| format!("bad port: {p}"))?;
                (h.to_string(), port)
            }
            _ => (authority.to_string(), 80),
        };
        if host.is_empty() {
            return Err("the address has no host".into());
        }
        let token = token
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string);
        Ok(Self {
            host,
            port,
            path: path.to_string(),
            token,
            timeout: Duration::from_secs(15),
        })
    }

    /// The same host and token, another path (for `/health` and similar).
    pub fn with_path(&self, path: &str) -> Self {
        Self {
            path: path.to_string(),
            ..self.clone()
        }
    }

    /// POSTs `body` as JSON and returns the reply.
    pub fn post_json(
        &self,
        body: &Value,
        extra_headers: &[(&str, &str)],
    ) -> Result<HttpReply, String> {
        let body = serde_json::to_vec(body).map_err(|e| e.to_string())?;
        self.request("POST", Some(&body), extra_headers)
    }

    /// GETs the endpoint's path.
    pub fn get(&self) -> Result<HttpReply, String> {
        self.request("GET", None, &[])
    }

    fn request(
        &self,
        method: &str,
        body: Option<&[u8]>,
        extra: &[(&str, &str)],
    ) -> Result<HttpReply, String> {
        let bare_host = self.host.trim_matches(|c| c == '[' || c == ']');
        let addr = (bare_host, self.port)
            .to_socket_addrs()
            .map_err(|e| format!("cannot resolve {}: {e}", self.host))?
            .next()
            .ok_or_else(|| format!("no address for {}", self.host))?;
        let mut stream = TcpStream::connect_timeout(&addr, self.timeout)
            .map_err(|e| format!("cannot reach {}:{}: {e}", self.host, self.port))?;
        stream.set_read_timeout(Some(self.timeout)).ok();
        stream.set_write_timeout(Some(self.timeout)).ok();

        let mut head = format!(
            "{method} {} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\n\
             Accept: application/json, text/event-stream\r\nUser-Agent: kathamo-remote\r\n",
            self.path, self.host, self.port
        );
        if let Some(token) = &self.token {
            head.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        for (k, v) in extra {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        if let Some(body) = body {
            head.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        head.push_str("\r\n");
        // One write: some hosts read the request in a single packet.
        let mut request = head.into_bytes();
        request.extend_from_slice(body.unwrap_or_default());
        stream.write_all(&request).map_err(|e| e.to_string())?;
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|e| format!("reply interrupted: {e}"))?;
        parse_http_reply(&raw)
    }
}

/// A parsed HTTP reply.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpReply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body as JSON. A server-sent-events body yields its last `data:` line.
    pub fn json(&self) -> Result<Value, String> {
        let text = self.text();
        let is_sse = self
            .header("content-type")
            .is_some_and(|t| t.starts_with("text/event-stream"));
        let payload = if is_sse {
            text.lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .last()
                .unwrap_or("")
                .trim()
                .to_string()
        } else {
            text
        };
        serde_json::from_str(&payload)
            .map_err(|e| format!("reply is not JSON (HTTP {}): {e}", self.status))
    }
}

fn parse_http_reply(raw: &[u8]) -> Result<HttpReply, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("the server closed the connection without a reply")?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or_else(|| format!("bad status line: {status_line}"))?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    let mut body = raw[split + 4..].to_vec();
    let chunked = headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked")
    });
    if chunked {
        body = decode_chunked(&body)?;
    }
    Ok(HttpReply {
        status,
        headers,
        body,
    })
}

fn decode_chunked(mut data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let line_end = data
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("truncated chunked body")?;
        let size_line = String::from_utf8_lossy(&data[..line_end]).into_owned();
        let size_text = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| format!("bad chunk size: {size_text}"))?;
        data = &data[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if data.len() < size {
            return Err("truncated chunk".into());
        }
        out.extend_from_slice(&data[..size]);
        data = data.get(size + 2..).unwrap_or(&[]);
    }
}

/// What a host says about one of its tools.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    /// The host marks the tool as only reading state (`readOnlyHint`).
    pub read_only: bool,
    /// The host marks the tool as able to destroy data (`destructiveHint`).
    pub destructive: bool,
    /// Argument names the tool requires.
    pub required: Vec<String>,
    pub input_schema: Value,
}

impl ToolInfo {
    fn from_json(tool: &Value) -> Option<Self> {
        let hint = |k: &str| {
            tool.pointer(&format!("/annotations/{k}"))
                .and_then(Value::as_bool)
        };
        let read_only = hint("readOnlyHint").unwrap_or(false);
        Some(Self {
            name: tool.get("name")?.as_str()?.to_string(),
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            read_only,
            // MCP's default for a tool without hints is "may be destructive".
            destructive: hint("destructiveHint").unwrap_or(!read_only),
            required: tool
                .pointer("/inputSchema/required")
                .and_then(Value::as_array)
                .map(|r| {
                    r.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
            input_schema: tool.get("inputSchema").cloned().unwrap_or(Value::Null),
        })
    }

    /// Safe to run with one tap from a phone: read-only and needs no arguments.
    pub fn one_tap_safe(&self) -> bool {
        self.read_only && !self.destructive && self.required.is_empty()
    }
}

/// A blocking MCP client over HTTP JSON-RPC ("streamable HTTP" transport).
#[derive(Debug, Clone)]
pub struct McpClient {
    pub endpoint: HttpEndpoint,
    session_id: Option<String>,
    next_id: u64,
    pub server_name: Option<String>,
}

impl McpClient {
    pub fn new(endpoint: HttpEndpoint) -> Self {
        Self {
            endpoint,
            session_id: None,
            next_id: 1,
            server_name: None,
        }
    }

    /// Sends one JSON-RPC request and returns its `result`, or the server's
    /// error message.
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let session = self.session_id.clone();
        let mut headers = Vec::new();
        if let Some(s) = session.as_deref() {
            headers.push(("Mcp-Session-Id", s));
        }
        let reply = self.endpoint.post_json(&body, &headers)?;
        if let Some(s) = reply.header("mcp-session-id") {
            self.session_id = Some(s.to_string());
        }
        // Setu answers a wrong token with an empty 404, to hide that it exists.
        let refused = reply.status == 401
            || reply.status == 403
            || (reply.status == 404 && reply.body.is_empty());
        if refused {
            return Err(format!(
                "the host refused the request (HTTP {}); check the token",
                reply.status
            ));
        }
        let value = reply.json()?;
        if let Some(err) = value.get("error") {
            let msg = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(format!("{method}: {msg}"));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{method}: reply has no result"))
    }

    /// MCP handshake. Returns the server's name.
    pub fn initialize(&mut self, client_name: &str) -> Result<String, String> {
        let result = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": client_name, "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        let name = result
            .pointer("/serverInfo/name")
            .and_then(Value::as_str)
            .unwrap_or("MCP server")
            .to_string();
        self.server_name = Some(name.clone());
        Ok(name)
    }

    pub fn list_tools(&mut self) -> Result<Vec<ToolInfo>, String> {
        let result = self.request("tools/list", json!({}))?;
        Ok(result
            .get("tools")
            .and_then(Value::as_array)
            .map(|tools| tools.iter().filter_map(ToolInfo::from_json).collect())
            .unwrap_or_default())
    }

    /// Calls a tool. A tool that reports `isError` becomes `Err` with its text.
    pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        let result = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )?;
        if result.get("isError").and_then(Value::as_bool) == Some(true) {
            return Err(tool_text(&result));
        }
        Ok(result)
    }
}

/// The readable text of a `tools/call` result: its text blocks joined, or the
/// structured content pretty-printed when there is no text.
pub fn tool_text(result: &Value) -> String {
    let texts: Vec<&str> = result
        .get("content")
        .and_then(Value::as_array)
        .map(|c| {
            c.iter()
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if !texts.is_empty() {
        return texts.join("\n");
    }
    let shown = result.get("structuredContent").unwrap_or(result);
    serde_json::to_string_pretty(shown).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn addresses_parse_and_https_is_refused_clearly() {
        let e = HttpEndpoint::parse("http://100.64.0.2:3000/mcp", Some(" tok ")).unwrap();
        assert_eq!(
            (e.host.as_str(), e.port, e.path.as_str()),
            ("100.64.0.2", 3000, "/mcp")
        );
        assert_eq!(e.token.as_deref(), Some("tok"));
        let e = HttpEndpoint::parse("desk.tailnet.ts.net", Some("")).unwrap();
        assert_eq!((e.port, e.path.as_str(), e.token), (80, "/", None));
        assert!(HttpEndpoint::parse("https://x/mcp", None)
            .unwrap_err()
            .contains("Tailscale"));
        assert!(HttpEndpoint::parse("http://:3000", None).is_err());
        assert!(HttpEndpoint::parse("http://h:99999", None).is_err());
    }

    #[test]
    fn chunked_and_event_stream_replies_decode() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/event-stream\r\n\r\n5\r\ndata:\r\n9\r\n {\"a\":1}\n\r\n0\r\n\r\n";
        let reply = parse_http_reply(raw).unwrap();
        assert_eq!(reply.status, 200);
        assert_eq!(reply.json().unwrap(), json!({"a": 1}));
    }

    #[test]
    fn tool_safety_follows_the_host_hints() {
        let t = |v: Value| ToolInfo::from_json(&v).unwrap();
        let list = t(json!({"name":"agent_list",
            "annotations":{"readOnlyHint":true,"destructiveHint":false}}));
        assert!(list.one_tap_safe());
        let read = t(json!({"name":"read_file","annotations":{"readOnlyHint":true},
            "inputSchema":{"required":["path"]}}));
        assert!(!read.one_tap_safe());
        let run = t(json!({"name":"run_terminal_command"}));
        assert!(run.destructive && !run.one_tap_safe());
    }

    /// Reads one whole HTTP request (headers plus `Content-Length` body).
    /// Closing a socket with unread input makes Windows reset the connection
    /// and drop the reply, so a fake host must read everything first.
    pub(crate) fn read_request(s: &mut std::net::TcpStream) -> String {
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = s.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if data.len() >= end + 4 + len {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&data).into_owned()
    }

    /// A one-shot fake MCP host on localhost: answers one request and returns it.
    fn fake_host(reply_body: Value) -> (HttpEndpoint, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let request = read_request(&mut s);
            let body = reply_body.to_string();
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nmcp-session-id: s1\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            s.write_all(reply.as_bytes()).unwrap();
            request
        });
        let url = format!("http://127.0.0.1:{port}/mcp");
        (HttpEndpoint::parse(&url, Some("secret")).unwrap(), handle)
    }

    #[test]
    fn mcp_calls_send_the_token_and_read_tool_text() {
        let (endpoint, host) = fake_host(json!({"jsonrpc":"2.0","id":1,"result":{
            "content":[{"type":"text","text":"2 agents online"}]}}));
        let mut mcp = McpClient::new(endpoint);
        let reply = mcp.call_tool("agent_list", json!({})).unwrap();
        assert_eq!(tool_text(&reply), "2 agents online");
        let request = host.join().unwrap();
        assert!(request.starts_with("POST /mcp HTTP/1.1"));
        assert!(request.contains("Authorization: Bearer secret"));
        assert!(request.contains("\"method\":\"tools/call\""));
        assert_eq!(mcp.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn server_errors_become_messages() {
        let (endpoint, host) = fake_host(
            json!({"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"no such tool"}}),
        );
        let err = McpClient::new(endpoint)
            .call_tool("nope", json!({}))
            .unwrap_err();
        assert!(err.contains("no such tool"));
        host.join().unwrap();
    }
}
