//! Setu Remote: control a Kathamo host app from a phone.
//!
//! Setu (and Kolpona services) publish their controls as MCP tools over HTTP.
//! This app connects to one, lists its tools, and runs the safe ones with a tap:
//! a tool runs only when the host marks it read-only and it needs no arguments.
//! Anything that writes files, runs commands or needs input stays on the PC.
//!
//! It exercises the Kathamo phone features a controller needs:
//! * on-screen keyboard on demand (`AppContext::set_text_input`),
//! * worker threads that wake the UI (`AppContext::waker`),
//! * saved settings in the app's private folder (`AppContext::data_dir`),
//! * the Android Back button (closes the keyboard, then the result, then the app),
//! * layout inside the status/gesture bars (`AppContext::safe_bounds`).
//!
//! Android: `pwsh tools/android/build_apk.ps1 -Crate kathamo-setu-remote
//! -Package studio.kalponic.setu.remote -Label "Setu Remote"`.
//! Desktop preview: `cargo run -p kathamo-setu-remote`.

use std::sync::{Arc, Mutex};

use kathamo::prelude::*;
use kathamo::remote::{json, tool_text, HttpEndpoint, McpClient, ToolInfo, Value};

const ROW_H: f32 = 54.0;
const SETTINGS_FILE: &str = "setu_remote.json";

/// The address a fresh install tries. The Android emulator reaches the PC's
/// localhost at 10.0.2.2; a real phone needs the PC's Tailscale name or address.
fn default_url() -> &'static str {
    if cfg!(target_os = "android") {
        "http://10.0.2.2:3000/mcp"
    } else {
        "http://127.0.0.1:3000/mcp"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Url,
    Token,
}

/// State shared with worker threads.
#[derive(Default)]
pub struct Shared {
    pub busy: bool,
    pub status: String,
    pub connected: bool,
    pub tools: Vec<ToolInfo>,
    pub client: Option<McpClient>,
    /// Title and text of the last tool result.
    pub result: Option<(String, String)>,
}

pub struct SetuRemote {
    pub url: String,
    pub token: String,
    pub focus: Option<Field>,
    pub shared: Arc<Mutex<Shared>>,
    pub list_scroll: f32,
    pub result_scroll: f32,
    waker: Option<UiWaker>,
    data_dir: Option<std::path::PathBuf>,
}

impl Default for SetuRemote {
    fn default() -> Self {
        Self {
            url: default_url().into(),
            token: String::new(),
            focus: None,
            shared: Arc::new(Mutex::new(Shared {
                status: "Not connected".into(),
                ..Shared::default()
            })),
            list_scroll: 0.0,
            result_scroll: 0.0,
            waker: None,
            data_dir: None,
        }
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Vec2::new(x, y), Vec2::new(w.max(1.0), h.max(1.0)))
}

fn text(buf: &mut CommandBuffer, value: &str, x: f32, y: f32, size: f32, color: Color, strong: bool) {
    buf.push(DrawCommand::Text { text: value.to_owned(), pos: Vec2::new(x, y), size, color, strong });
}

fn panel(buf: &mut CommandBuffer, r: Rect, fill: Color, edge: Color, radius: f32) {
    buf.push(DrawCommand::Rect { rect: r, fill, radius: CornerRadius::same(radius), stroke: Stroke::new(1.0, edge) });
}

/// Breaks text into lines of at most `width` characters (rough monospace fit).
fn wrap(textv: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut out = Vec::new();
    for line in textv.lines() {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            out.push(String::new());
        }
        for chunk in chars.chunks(width) {
            out.push(chunk.iter().collect());
        }
    }
    out
}

/// Screen regions, laid out inside the safe area.
pub struct Layout {
    pub url: Rect,
    pub token: Rect,
    pub connect: Rect,
    pub status_y: f32,
    pub tools: Rect,
    pub result: Option<Rect>,
    pub result_close: Option<Rect>,
}

impl Layout {
    pub fn of(safe: Rect, has_result: bool) -> Self {
        let pad = 16.0;
        let x = safe.min.x + pad;
        let w = safe.width() - pad * 2.0;
        let y = safe.min.y;
        let url = rect(x, y + 74.0, w, 48.0);
        let token = rect(x, y + 130.0, w, 48.0);
        let connect = rect(x, y + 188.0, w, 50.0);
        let status_y = y + 248.0;
        let top = y + 276.0;
        let bottom = safe.max.y - pad;
        let avail = (bottom - top).max(ROW_H * 2.0);
        if has_result {
            let list_h = (avail * 0.42).max(ROW_H);
            let tools = rect(x, top, w, list_h);
            let result = rect(x, top + list_h + 10.0, w, avail - list_h - 10.0);
            let close = rect(result.max.x - 52.0, result.min.y + 4.0, 48.0, 36.0);
            Self { url, token, connect, status_y, tools, result: Some(result), result_close: Some(close) }
        } else {
            let tools = rect(x, top, w, avail);
            Self { url, token, connect, status_y, tools, result: None, result_close: None }
        }
    }
}

impl SetuRemote {
    fn layout(&self, ctx: &AppContext) -> Layout {
        let has_result = self.shared.lock().map(|s| s.result.is_some()).unwrap_or(false);
        Layout::of(ctx.safe_bounds(), has_result)
    }

    fn settings_path(&self) -> Option<std::path::PathBuf> {
        self.data_dir.as_ref().map(|d| d.join(SETTINGS_FILE))
    }

    fn load_settings(&mut self) {
        let Some(path) = self.settings_path() else { return };
        let Ok(raw) = std::fs::read_to_string(path) else { return };
        let Ok(v) = raw.parse::<Value>() else { return };
        if let Some(url) = v.get("url").and_then(Value::as_str) {
            self.url = url.to_string();
        }
        if let Some(token) = v.get("token").and_then(Value::as_str) {
            self.token = token.to_string();
        }
    }

    /// Kept in the app's private storage; other apps cannot read it.
    fn save_settings(&self) {
        let Some(path) = self.settings_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body = json!({ "url": self.url, "token": self.token }).to_string();
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("Setu Remote: cannot save settings: {e}");
        }
    }

    fn set_focus(&mut self, focus: Option<Field>, ctx: &mut AppContext) {
        self.focus = focus;
        ctx.set_text_input(focus.is_some());
    }

    fn wake(&self) {
        if let Some(w) = &self.waker {
            w.wake();
        }
    }

    /// Connects on a worker thread: MCP handshake, then the tool list.
    pub fn connect(&mut self) {
        let endpoint = match HttpEndpoint::parse(&self.url, Some(&self.token)) {
            Ok(e) => e,
            Err(e) => {
                if let Ok(mut s) = self.shared.lock() {
                    s.status = e;
                    s.connected = false;
                }
                return;
            }
        };
        self.save_settings();
        {
            let Ok(mut s) = self.shared.lock() else { return };
            if s.busy {
                return;
            }
            s.busy = true;
            s.status = format!("Connecting to {}:{} ...", endpoint.host, endpoint.port);
        }
        let shared = self.shared.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let mut client = McpClient::new(endpoint);
            let outcome = client
                .initialize("kathamo-setu-remote")
                .and_then(|name| client.list_tools().map(|tools| (name, tools)));
            if let Ok(mut s) = shared.lock() {
                s.busy = false;
                match outcome {
                    Ok((name, tools)) => {
                        let safe = tools.iter().filter(|t| t.one_tap_safe()).count();
                        s.status = format!("Connected to {name}: {} tools, {safe} safe to tap", tools.len());
                        s.connected = true;
                        s.tools = tools;
                        s.client = Some(client);
                    }
                    Err(e) => {
                        s.status = e;
                        s.connected = false;
                        s.tools.clear();
                        s.client = None;
                    }
                }
            }
            if let Some(w) = waker {
                w.wake();
            }
        });
        self.wake();
    }

    /// Runs one tool on a worker thread, only if it is one-tap safe.
    pub fn run_tool(&mut self, index: usize) {
        let (tool, client) = {
            let Ok(mut s) = self.shared.lock() else { return };
            let Some(tool) = s.tools.get(index).cloned() else { return };
            if !tool.one_tap_safe() {
                let why = if tool.required.is_empty() {
                    "it can change things on the PC".to_string()
                } else {
                    format!("it needs input ({})", tool.required.join(", "))
                };
                s.status = format!("{} stays on the PC: {why}", tool.name);
                return;
            }
            if s.busy {
                return;
            }
            let Some(client) = s.client.clone() else { return };
            s.busy = true;
            s.status = format!("Running {} ...", tool.name);
            (tool, client)
        };
        self.result_scroll = 0.0;
        let shared = self.shared.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let mut client = client;
            let outcome = client.call_tool(&tool.name, json!({}));
            if let Ok(mut s) = shared.lock() {
                s.busy = false;
                match outcome {
                    Ok(reply) => {
                        s.status = format!("{} done", tool.name);
                        s.result = Some((tool.name.clone(), tool_text(&reply)));
                    }
                    Err(e) => {
                        s.status = format!("{} failed", tool.name);
                        s.result = Some((tool.name.clone(), e));
                    }
                }
            }
            if let Some(w) = waker {
                w.wake();
            }
        });
        self.wake();
    }

    fn tools_max_scroll(&self, list: Rect) -> f32 {
        let n = self.shared.lock().map(|s| s.tools.len()).unwrap_or(0);
        (n as f32 * ROW_H - list.height()).max(0.0)
    }

    fn result_lines(&self, r: Rect) -> Vec<String> {
        let body = self
            .shared
            .lock()
            .ok()
            .and_then(|s| s.result.as_ref().map(|(_, t)| t.clone()))
            .unwrap_or_default();
        wrap(&body, ((r.width() - 24.0) / 6.6) as usize)
    }

    fn result_max_scroll(&self, r: Rect) -> f32 {
        let lines = self.result_lines(r).len() as f32;
        (lines * 15.0 - (r.height() - 44.0)).max(0.0)
    }
}

impl KathamoApp for SetuRemote {
    fn title(&self) -> &str {
        "Setu Remote"
    }

    fn default_size(&self) -> (u32, u32) {
        (412, 892)
    }

    fn init(&mut self, ctx: &mut AppContext) {
        self.waker = ctx.waker();
        self.data_dir = ctx.data_dir().map(|p| p.to_path_buf());
        self.load_settings();
    }

    fn update(&mut self, ctx: &mut AppContext) {
        // The user closed the keyboard outside the app: leave the field.
        if self.focus.is_some() && !ctx.text_input_wanted() {
            self.focus = None;
        }
        let safe = ctx.safe_bounds();
        let l = self.layout(ctx);
        let p = ctx.theme;
        let (status, busy, connected, tools, result) = match self.shared.lock() {
            Ok(s) => (s.status.clone(), s.busy, s.connected, s.tools.clone(), s.result.clone()),
            Err(_) => ("internal error".into(), false, false, Vec::new(), None),
        };
        let result_lines = l.result.map(|r| self.result_lines(r)).unwrap_or_default();
        let buf = &mut ctx.cmd_buf;

        let x = safe.min.x + 16.0;
        text(buf, "Setu Remote", x, safe.min.y + 14.0, 26.0, p.text, true);
        text(buf, "Control your PC's Setu from this phone", x, safe.min.y + 50.0, 12.0, p.text_muted, false);

        for (field, r, label, value) in [
            (Field::Url, l.url, "Host", self.url.clone()),
            (Field::Token, l.token, "Token", "•".repeat(self.token.chars().count())),
        ] {
            let focused = self.focus == Some(field);
            panel(buf, r, p.elevated, if focused { p.accent } else { p.border }, 12.0);
            text(buf, label, r.min.x + 12.0, r.min.y + 6.0, 10.0, p.text_muted, false);
            let shown = if value.is_empty() && field == Field::Token { "none (Setu on this PC needs none)".to_string() } else { value };
            let max_chars = ((r.width() - 30.0) / 7.4) as usize;
            let count = shown.chars().count();
            let visible: String = if count > max_chars { shown.chars().skip(count - max_chars).collect() } else { shown };
            let muted = field == Field::Token && self.token.is_empty();
            text(buf, &visible, r.min.x + 12.0, r.min.y + 22.0, 14.0, if muted { p.text_muted } else { p.text }, false);
            if focused {
                let cx = r.min.x + 12.0 + visible.chars().count().min(max_chars) as f32 * 7.4 + 1.0;
                buf.push(DrawCommand::Rect {
                    rect: rect(cx.min(r.max.x - 8.0), r.min.y + 22.0, 2.0, 18.0),
                    fill: p.accent,
                    radius: CornerRadius::same(1.0),
                    stroke: Stroke::new(0.0, p.border),
                });
            }
        }

        let c = l.connect;
        let label = if busy { "Working ..." } else if connected { "Reconnect" } else { "Connect" };
        panel(buf, c, if busy { p.border } else { p.accent }, p.accent_hover, 14.0);
        text(buf, label, c.min.x + 16.0, c.min.y + 15.0, 16.0, p.canvas, true);
        let status_color = if connected { p.text } else { p.text_muted };
        let status_short: String = status.chars().take(((safe.width() - 32.0) / 6.4) as usize).collect();
        text(buf, &status_short, x, l.status_y, 11.0, status_color, false);

        let list = l.tools;
        panel(buf, list, p.surface, p.border, 14.0);
        if tools.is_empty() {
            text(buf, "Connect to see what the PC lets you do.", list.min.x + 16.0, list.min.y + 18.0, 12.0, p.text_muted, false);
        }
        buf.push(DrawCommand::ClipPush { rect: list });
        for (i, tool) in tools.iter().enumerate() {
            let y = list.min.y + i as f32 * ROW_H - self.list_scroll;
            if y + ROW_H < list.min.y || y > list.max.y {
                continue;
            }
            if i > 0 {
                buf.push(DrawCommand::Line {
                    start: Vec2::new(list.min.x + 14.0, y),
                    end: Vec2::new(list.max.x - 14.0, y),
                    stroke: Stroke::new(1.0, p.border.with_alpha(110)),
                });
            }
            let safe_tool = tool.one_tap_safe();
            text(buf, &tool.name, list.min.x + 16.0, y + 10.0, 14.0, if safe_tool { p.text } else { p.text_muted }, true);
            let hint = if safe_tool {
                "Tap to run (read-only)".to_string()
            } else if !tool.required.is_empty() && tool.read_only {
                format!("Needs {}", tool.required.join(", "))
            } else {
                "Not marked read-only: PC only".to_string()
            };
            text(buf, &hint, list.min.x + 16.0, y + 31.0, 10.5, if safe_tool { p.accent } else { p.text_muted }, false);
        }
        buf.push(DrawCommand::ClipPop);

        if let (Some(r), Some(close), Some((title, _))) = (l.result, l.result_close, result.as_ref()) {
            panel(buf, r, p.elevated, p.border, 14.0);
            text(buf, title, r.min.x + 14.0, r.min.y + 12.0, 14.0, p.accent, true);
            text(buf, "Close", close.min.x + 4.0, close.min.y + 10.0, 12.0, p.text_muted, false);
            let body = rect(r.min.x, r.min.y + 40.0, r.width(), r.height() - 44.0);
            buf.push(DrawCommand::ClipPush { rect: body });
            for (i, line) in result_lines.iter().enumerate() {
                let y = body.min.y + i as f32 * 15.0 - self.result_scroll;
                if y + 15.0 < body.min.y || y > body.max.y {
                    continue;
                }
                text(buf, line, r.min.x + 12.0, y, 11.0, p.text, false);
            }
            buf.push(DrawCommand::ClipPop);
        }
    }

    fn handle_mouse_down(&mut self, pos: Vec2, _b: PointerButton, ctx: &mut AppContext) -> bool {
        let l = self.layout(ctx);
        if l.url.contains(pos) {
            self.set_focus(Some(Field::Url), ctx);
            return true;
        }
        if l.token.contains(pos) {
            self.set_focus(Some(Field::Token), ctx);
            return true;
        }
        self.set_focus(None, ctx);
        if l.connect.contains(pos) {
            self.connect();
            return true;
        }
        if let Some(close) = l.result_close {
            if close.contains(pos) {
                if let Ok(mut s) = self.shared.lock() {
                    s.result = None;
                }
                return true;
            }
        }
        if l.tools.contains(pos) {
            let index = ((pos.y - l.tools.min.y + self.list_scroll) / ROW_H) as usize;
            self.run_tool(index);
            // A tap selects; a drag that follows still scrolls the list.
            return false;
        }
        false
    }

    fn handle_scroll(&mut self, delta: Vec2, pos: Vec2, ctx: &mut AppContext) -> bool {
        let l = self.layout(ctx);
        if let Some(r) = l.result {
            if r.contains(pos) {
                self.result_scroll = (self.result_scroll - delta.y * 20.0).clamp(0.0, self.result_max_scroll(r));
                return true;
            }
        }
        if l.tools.contains(pos) {
            self.list_scroll = (self.list_scroll - delta.y * 20.0).clamp(0.0, self.tools_max_scroll(l.tools));
            return true;
        }
        false
    }

    fn handle_text_input(&mut self, typed: &str, _ctx: &mut AppContext) -> bool {
        let target = match self.focus {
            Some(Field::Url) => &mut self.url,
            Some(Field::Token) => &mut self.token,
            None => return false,
        };
        target.extend(typed.chars().filter(|c| !c.is_control()));
        true
    }

    fn handle_key_down(&mut self, key: Key, _m: Modifiers, ctx: &mut AppContext) -> bool {
        match key {
            Key::Backspace if self.focus.is_some() => {
                match self.focus {
                    Some(Field::Url) => self.url.pop(),
                    Some(Field::Token) => self.token.pop(),
                    None => None,
                };
                true
            }
            Key::Enter if self.focus.is_some() => {
                let was_url = self.focus == Some(Field::Url);
                self.set_focus(None, ctx);
                if was_url {
                    self.connect();
                }
                true
            }
            // Back: close the keyboard, then the result, then leave the app.
            Key::Custom(name) if name == "Back" => {
                if self.focus.is_some() {
                    self.set_focus(None, ctx);
                    return true;
                }
                if let Ok(mut s) = self.shared.lock() {
                    if s.result.take().is_some() {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    if let Err(error) = run_android(SetuRemote::default(), app) {
        eprintln!("Setu Remote stopped: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn runner(dir: Option<std::path::PathBuf>, wakes: Arc<AtomicUsize>) -> HeadlessAppRunner<SetuRemote> {
        let waker = UiWaker::new(move || {
            wakes.fetch_add(1, Ordering::SeqCst);
        });
        let mut r = HeadlessAppRunner::with_platform(SetuRemote::default(), Some(waker), dir);
        r.step_frame();
        r
    }

    #[test]
    fn tapping_a_field_asks_for_the_keyboard_and_back_closes_it() {
        let mut r = runner(None, Arc::default());
        let l = r.app.layout(&r.ctx);
        r.click_at(l.token.center(), PointerButton::Primary);
        assert_eq!(r.app.focus, Some(Field::Token));
        assert!(r.ctx.text_input_wanted());
        r.app.handle_text_input("ab\u{8}c", &mut r.ctx);
        r.app.handle_key_down(Key::Backspace, Modifiers::NONE, &mut r.ctx);
        assert_eq!(r.app.token, "ab");
        assert!(r.app.handle_key_down(Key::Custom("Back".into()), Modifiers::NONE, &mut r.ctx));
        assert!(!r.ctx.text_input_wanted());
        // Nothing left to close: Back goes to the system and leaves the app.
        assert!(!r.app.handle_key_down(Key::Custom("Back".into()), Modifiers::NONE, &mut r.ctx));
    }

    #[test]
    fn https_is_refused_before_any_network_use() {
        let mut r = runner(None, Arc::default());
        r.app.url = "https://example.com/mcp".into();
        r.app.connect();
        let s = r.app.shared.lock().unwrap();
        assert!(!s.busy && s.status.contains("Tailscale"));
    }

    /// A tiny fake Setu: answers `n` MCP requests from canned replies by method.
    fn fake_setu(n: usize) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/mcp", listener.local_addr().unwrap().port());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for _ in 0..n {
                let (mut s, _) = listener.accept().unwrap();
                let req = read_request(&mut s);
                let result = if req.contains("\"initialize\"") {
                    json!({"serverInfo":{"name":"Setu"}})
                } else if req.contains("tools/list") {
                    json!({"tools":[
                        {"name":"agent_list","annotations":{"readOnlyHint":true,"destructiveHint":false}},
                        {"name":"run_terminal_command","inputSchema":{"required":["command"]}}]})
                } else {
                    json!({"content":[{"type":"text","text":"pothik: online"}]})
                };
                seen.push(req);
                let body = json!({"jsonrpc":"2.0","id":1,"result":result}).to_string();
                let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                s.write_all(reply.as_bytes()).unwrap();
            }
            seen
        });
        (url, handle)
    }

    /// Reads a whole HTTP request so closing the socket cannot reset it.
    fn read_request(s: &mut std::net::TcpStream) -> String {
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

    fn wait_idle(app: &SetuRemote) {
        let start = Instant::now();
        while app.shared.lock().unwrap().busy {
            assert!(start.elapsed() < Duration::from_secs(10), "worker never finished");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn connect_lists_tools_runs_only_safe_ones_and_saves_settings() {
        let dir = std::env::temp_dir().join(format!("setu-remote-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let wakes = Arc::new(AtomicUsize::new(0));
        let mut r = runner(Some(dir.clone()), wakes.clone());
        let (url, host) = fake_setu(3);
        r.app.url = url.clone();
        r.app.token = "t0k".into();
        r.app.connect();
        wait_idle(&r.app);
        {
            let s = r.app.shared.lock().unwrap();
            assert!(s.connected, "{}", s.status);
            assert_eq!(s.tools.len(), 2);
            assert!(s.status.contains("1 safe"));
        }
        // The worker woke the UI thread when the reply landed.
        assert!(wakes.load(Ordering::SeqCst) >= 1);

        // The command runner is refused locally; no request reaches the host.
        r.app.run_tool(1);
        assert!(r.app.shared.lock().unwrap().status.contains("stays on the PC"));
        r.app.run_tool(0);
        wait_idle(&r.app);
        let result = r.app.shared.lock().unwrap().result.clone();
        assert_eq!(result, Some(("agent_list".into(), "pothik: online".into())));

        let requests = host.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests.iter().all(|q| q.contains("Authorization: Bearer t0k")));
        assert!(!requests.iter().any(|q| q.contains("run_terminal_command\",\"arguments")));

        // Settings survive a restart.
        let r2 = runner(Some(dir.clone()), Arc::default());
        assert_eq!((r2.app.url.as_str(), r2.app.token.as_str()), (url.as_str(), "t0k"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn layout_leaves_room_for_the_result_and_stays_in_the_safe_area() {
        let safe = Rect::from_min_max(Vec2::new(0.0, 48.0), Vec2::new(412.0, 868.0));
        let l = Layout::of(safe, true);
        let res = l.result.unwrap();
        assert!(l.url.min.y > 48.0 && res.max.y <= 868.0);
        assert!(l.tools.max.y < res.min.y && l.tools.height() >= ROW_H);
    }
}
