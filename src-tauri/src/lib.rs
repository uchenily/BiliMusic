use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
  collections::HashMap,
  fs,
  io::{Read, Write},
  net::{TcpListener, TcpStream},
  path::PathBuf,
  sync::Arc,
};
use tauri::{
  menu::{Menu, MenuItem, PredefinedMenuItem},
  tray::TrayIconBuilder,
  AppHandle, Emitter, Manager, State, WindowEvent,
};
use tokio::sync::Mutex;

const BILI_API: &str = "https://api.bilibili.com";
const BILI_PASSPORT: &str = "https://passport.bilibili.com";
const BILI_REFERER: &str = "https://www.bilibili.com";
const OIAPI_QQ_LYRIC: &str = "https://www.oiapi.net/api/QQMusicLyric";

#[derive(Clone)]
struct AppState {
  client: Client,
  cookies: Arc<Mutex<String>>,
  webdav: Arc<Mutex<Option<WebdavConfig>>>,
  data_dir: PathBuf,
}

#[derive(Clone, Serialize, Deserialize)]
struct WebdavConfig {
  url: String,
  username: String,
  password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WebdavConfigInfo {
  url: String,
  username: String,
  configured: bool,
}

#[derive(Serialize)]
struct WebdavResult {
  ok: bool,
  status: u16,
  etag: Option<String>,
  content: Option<String>,
  message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CookiesInfo {
  is_logged_in: bool,
  sessdata: String,
  bili_jct: String,
  dede_user_id: String,
}

fn app_file(state: &AppState, name: &str) -> PathBuf {
  state.data_dir.join(name)
}

fn load_text(path: &PathBuf) -> Option<String> {
  fs::read_to_string(path).ok()
}

fn save_text(path: &PathBuf, content: &str) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  fs::write(path, content).map_err(|e| e.to_string())
}

fn parse_cookie_pair(raw: &str) -> Option<(String, String)> {
  let pair = raw.split(';').next()?.trim();
  let (name, value) = pair.split_once('=')?;
  Some((name.trim().to_string(), value.trim().to_string()))
}

fn cookie_value(cookie_header: &str, name: &str) -> String {
  cookie_header
    .split(';')
    .filter_map(|part| part.trim().split_once('='))
    .find(|(key, _)| *key == name)
    .map(|(_, value)| value.to_string())
    .unwrap_or_default()
}

async fn cookie_header(state: &AppState) -> String {
  state.cookies.lock().await.clone()
}

async fn set_cookie(state: &AppState, name: String, value: String) -> Result<(), String> {
  let mut cookies = state.cookies.lock().await;
  let mut pairs: Vec<(String, String)> = cookies
    .split(';')
    .filter_map(parse_cookie_pair)
    .filter(|(key, _)| key != &name)
    .collect();
  pairs.push((name, value));
  *cookies = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
  save_text(&app_file(state, "bilibili-cookies.txt"), &cookies)
}

async fn ensure_bilibili_device_cookies(state: &AppState, user_agent: Option<&str>) {
  let existing = cookie_header(state).await;
  if existing.contains("buvid3=") && existing.contains("b_nut=") {
    return;
  }

  let Ok(client) = Client::builder()
    .redirect(reqwest::redirect::Policy::limited(5))
    .build()
  else {
    return;
  };
  let mut request = client
    .get("https://www.bilibili.com/")
    .header("Referer", BILI_REFERER)
    .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8");
  if let Some(user_agent) = user_agent.filter(|value| !value.is_empty()) {
    request = request.header("User-Agent", user_agent);
  }
  let Ok(response) = request.send().await else {
    return;
  };

  for value in response.headers().get_all("set-cookie").iter() {
    if let Ok(raw) = value.to_str() {
      if let Some((name, cookie)) = parse_cookie_pair(raw) {
        let _ = set_cookie(state, name, cookie).await;
      }
    }
  }
}

async fn request_bili(
  state: &AppState,
  path: &str,
  user_agent: Option<&str>,
) -> Result<reqwest::Response, String> {
  if !path.starts_with('/') || path.contains('\\') || path.contains("://") {
    return Err("Invalid Bilibili API path".to_string());
  }
  ensure_bilibili_device_cookies(state, user_agent).await;
  let url = format!("{BILI_API}{path}");
  let mut request = state.client.get(url)
    .header("Referer", BILI_REFERER)
    .header("Origin", "https://www.bilibili.com")
    .header("Accept", "application/json, text/plain, */*");
  if let Some(user_agent) = user_agent.filter(|value| !value.is_empty()) {
    request = request.header("User-Agent", user_agent);
  }
  let cookies = cookie_header(state).await;
  if !cookies.is_empty() {
    request = request.header("Cookie", cookies);
  }
  request.send().await.map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
async fn bili_raw(
  state: State<'_, AppState>,
  path: String,
  user_agent: Option<String>,
) -> Result<Value, String> {
  let response = request_bili(&state, &path, user_agent.as_deref()).await?;
  response.json::<Value>().await.map_err(|e| e.to_string())
}

fn is_https_url(raw: &str) -> bool {
  reqwest::Url::parse(raw)
    .map(|url| url.scheme() == "https")
    .unwrap_or(false)
}

#[tauri::command(rename_all = "camelCase")]
async fn fetch_subtitle(
  state: State<'_, AppState>,
  url: String,
  user_agent: Option<String>,
) -> Result<Value, String> {
  if !is_https_url(&url) { return Err("Invalid subtitle URL".to_string()) }
  let client = Client::builder()
    .redirect(reqwest::redirect::Policy::limited(5))
    .build()
    .map_err(|e| e.to_string())?;
  let mut request = client
    .get(url)
    .header("Referer", BILI_REFERER)
    .header("Origin", "https://www.bilibili.com")
    .header("Accept", "application/json, text/plain, */*")
    .header("Accept-Language", "zh-CN,zh;q=0.9");
  if let Some(user_agent) = user_agent.filter(|value| !value.is_empty()) {
    request = request.header("User-Agent", user_agent);
  }
  let cookies = cookie_header(&state).await;
  if !cookies.is_empty() { request = request.header("Cookie", cookies); }
  let response = request.send().await.map_err(|e| e.to_string())?;
  if !response.status().is_success() { return Err(format!("Subtitle request failed: {}", response.status())) }
  response.json::<Value>().await.map_err(|e| e.to_string())
}

const MEDIA_PROXY_ADDR: &str = "127.0.0.1:17823";

fn write_proxy_headers(
  stream: &mut TcpStream,
  status: u16,
  reason: &str,
  content_type: &str,
  content_length: Option<&str>,
  headers: &[(String, String)],
) {
  let mut response = format!(
    "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\nConnection: close\r\n",
  );
  if let Some(content_length) = content_length {
    response.push_str(&format!("Content-Length: {content_length}\r\n"));
  }
  for (name, value) in headers {
    response.push_str(&format!("{name}: {value}\r\n"));
  }
  response.push_str("\r\n");
  let _ = stream.write_all(response.as_bytes());
}

fn write_proxy_response(
  stream: &mut TcpStream,
  status: u16,
  reason: &str,
  content_type: &str,
  headers: &[(String, String)],
  body: &[u8],
) {
  write_proxy_headers(stream, status, reason, content_type, Some(&body.len().to_string()), headers);
  let _ = stream.write_all(body);
}

fn parse_proxy_request(stream: &mut TcpStream) -> Option<(String, HashMap<String, String>)> {
  let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
  let mut buffer = Vec::new();
  let mut chunk = [0_u8; 4096];
  loop {
    let count = stream.read(&mut chunk).ok()?;
    if count == 0 { return None; }
    buffer.extend_from_slice(&chunk[..count]);
    if buffer.windows(4).any(|window| window == b"\r\n\r\n") { break; }
    if buffer.len() > 64 * 1024 { return None; }
  }
  let request = String::from_utf8_lossy(&buffer);
  let mut lines = request.split("\r\n");
  let mut request_line = lines.next()?.split_whitespace();
  let method = request_line.next()?;
  if method != "GET" { return None; }
  let path = request_line.next()?.to_string();
  let mut headers = HashMap::new();
  for line in lines {
    if line.is_empty() { break; }
    if let Some((name, value)) = line.split_once(':') {
      headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
  }
  Some((path, headers))
}

fn proxy_target(path: &str) -> Option<(bool, String)> {
  let url = reqwest::Url::parse(&format!("http://localhost{path}")).ok()?;
  let target = url.query_pairs().find(|(key, _)| key == "url")?.1.into_owned();
  let is_audio = url.path() == "/audio";
  if url.path() != "/image" && !is_audio { return None; }
  is_https_url(&target).then_some((is_audio, target))
}

fn serve_media_proxy(mut stream: TcpStream) {
  let Some((path, headers)) = parse_proxy_request(&mut stream) else {
    write_proxy_response(&mut stream, 400, "Bad Request", "text/plain; charset=utf-8", &[], b"invalid proxy request");
    return;
  };
  let Some((is_audio, target)) = proxy_target(&path) else {
    write_proxy_response(&mut stream, 400, "Bad Request", "text/plain; charset=utf-8", &[], b"invalid HTTPS media URL");
    return;
  };
  let Ok(client) = reqwest::blocking::Client::builder()
    .redirect(reqwest::redirect::Policy::limited(5))
    .build()
  else {
    write_proxy_response(&mut stream, 500, "Internal Server Error", "text/plain; charset=utf-8", &[], b"media client unavailable");
    return;
  };
  let mut request = client.get(target)
    .header("Referer", BILI_REFERER)
    .header("Origin", BILI_REFERER);
  if let Some(value) = headers.get("user-agent") { request = request.header("User-Agent", value); }
  if let Some(value) = headers.get("accept") { request = request.header("Accept", value); }
  if is_audio {
    if let Some(value) = headers.get("range") { request = request.header("Range", value); }
  }
  let Ok(mut response) = request.send() else {
    write_proxy_response(&mut stream, 502, "Bad Gateway", "text/plain; charset=utf-8", &[], b"media request failed");
    return;
  };
  let status = response.status();
  if !status.is_success() {
    write_proxy_response(&mut stream, status.as_u16(), status.canonical_reason().unwrap_or("Upstream Error"), "text/plain; charset=utf-8", &[], b"media request rejected");
    return;
  }
  let content_type = response.headers()
    .get(reqwest::header::CONTENT_TYPE)
    .and_then(|value| value.to_str().ok())
    .unwrap_or(if is_audio { "audio/mp4" } else { "image/jpeg" })
    .to_string();
  let content_length = response.headers()
    .get(reqwest::header::CONTENT_LENGTH)
    .and_then(|value| value.to_str().ok())
    .map(str::to_string);
  let mut response_headers = Vec::new();
  if is_audio {
    response_headers.push(("Accept-Ranges".to_string(), "bytes".to_string()));
  }
  if let Some(value) = response.headers().get(reqwest::header::CONTENT_RANGE).and_then(|value| value.to_str().ok()) {
    response_headers.push(("Content-Range".to_string(), value.to_string()));
  }
  if is_audio {
    write_proxy_headers(
      &mut stream,
      status.as_u16(),
      status.canonical_reason().unwrap_or("OK"),
      &content_type,
      content_length.as_deref(),
      &response_headers,
    );
    let _ = std::io::copy(&mut response, &mut stream);
    return;
  }
  let Ok(body) = response.bytes() else {
    write_proxy_response(&mut stream, 502, "Bad Gateway", "text/plain; charset=utf-8", &[], b"media body unavailable");
    return;
  };
  write_proxy_response(&mut stream, status.as_u16(), status.canonical_reason().unwrap_or("OK"), &content_type, &response_headers, &body);
}

fn start_media_proxy() -> Result<(), String> {
  let listener = TcpListener::bind(MEDIA_PROXY_ADDR).map_err(|error| format!("Unable to start media proxy: {error}"))?;
  std::thread::Builder::new()
    .name("bili-media-proxy".to_string())
    .spawn(move || {
      for stream in listener.incoming().flatten() {
        let _ = std::thread::Builder::new()
          .name("bili-media-request".to_string())
          .spawn(|| serve_media_proxy(stream));
      }
    })
    .map_err(|error| format!("Unable to start media proxy thread: {error}"))?;
  Ok(())
}

fn is_audio_url(raw: &str) -> bool {
  is_https_url(raw)
}

fn safe_file_name(filename: &str) -> String {
  PathBuf::from(filename)
    .file_name()
    .and_then(|name| name.to_str())
    .filter(|name| !name.is_empty())
    .unwrap_or("audio.m4a")
    .to_string()
}

async fn fetch_audio_bytes(state: &AppState, audio_url: &str) -> Result<Vec<u8>, String> {
  if !is_audio_url(audio_url) { return Err("Invalid audio URL".to_string()) }
  let response = state.client.get(audio_url)
    .header("Referer", BILI_REFERER)
    .header("Origin", "https://www.bilibili.com")
    .send()
    .await
    .map_err(|e| e.to_string())?;
  if !response.status().is_success() { return Err(format!("Audio request failed: {}", response.status())) }
  response.bytes().await.map(|bytes| bytes.to_vec()).map_err(|e| e.to_string())
}

async fn write_audio_file(state: &AppState, audio_url: &str, directory: &str, filename: String) -> Result<Value, String> {
  let bytes = fetch_audio_bytes(state, audio_url).await?;
  let target_dir = state.data_dir.join(directory);
  fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
  let file_path = target_dir.join(safe_file_name(&filename));
  fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;
  Ok(json!({ "filePath": file_path.to_string_lossy(), "size": bytes.len() }))
}

#[tauri::command(rename_all = "camelCase")]
async fn download_audio(state: State<'_, AppState>, audioUrl: String, filename: String) -> Result<Value, String> {
  write_audio_file(&state, &audioUrl, "downloads", filename).await
}

async fn lyric_json(url: String) -> Option<Value> {
  let response = reqwest::Client::new().get(url).timeout(std::time::Duration::from_secs(9)).send().await.ok()?;
  if !response.status().is_success() { return None }
  response.json::<Value>().await.ok()
}

#[tauri::command(rename_all = "camelCase")]
async fn lyrics_search(keyword: String, page: u32, limit: u32) -> Result<Vec<Value>, String> {
  let q = keyword.trim();
  if q.is_empty() { return Ok(Vec::new()) }
  let mut url = reqwest::Url::parse(OIAPI_QQ_LYRIC).map_err(|e| e.to_string())?;
  url.query_pairs_mut().append_pair("keyword", q).append_pair("page", &page.to_string()).append_pair("limit", &limit.to_string()).append_pair("type", "json");
  let Some(json) = lyric_json(url.to_string()).await else { return Ok(Vec::new()) };
  Ok(json.get("data").and_then(Value::as_array).cloned().unwrap_or_default())
}

#[tauri::command(rename_all = "camelCase")]
async fn lyrics_get(id: String, format: String) -> Result<Option<Value>, String> {
  let id = id.trim();
  if id.is_empty() { return Ok(None) }
  let mut url = reqwest::Url::parse(OIAPI_QQ_LYRIC).map_err(|e| e.to_string())?;
  url.query_pairs_mut().append_pair("id", id).append_pair("format", &format).append_pair("type", "json");
  let Some(json) = lyric_json(url.to_string()).await else { return Ok(None) };
  if json.get("code").and_then(Value::as_i64) != Some(1) { return Ok(None) }
  if let Some(content) = json.get("data").and_then(Value::as_str) { return Ok(Some(json!({"content": content}))) }
  if json.get("data").map(Value::is_object).unwrap_or(false) { return Ok(json.get("data").cloned()) }
  if let Some(message) = json.get("message").and_then(Value::as_str).filter(|m| m.contains('[')) { return Ok(Some(json!({"content": message}))) }
  Ok(None)
}

#[tauri::command(rename_all = "camelCase")]
async fn qr_generate(state: State<'_, AppState>) -> Result<Value, String> {
  let response = state.client.get(format!("{BILI_PASSPORT}/x/passport-login/web/qrcode/generate"))
    .header("Referer", format!("{BILI_PASSPORT}/login")).send().await.map_err(|e| e.to_string())?;
  let data: Value = response.json().await.map_err(|e| e.to_string())?;
  if data.get("code").and_then(Value::as_i64) != Some(0) { return Err(data.get("message").and_then(Value::as_str).unwrap_or("二维码生成失败").to_string()) }
  Ok(json!({ "url": data["data"]["url"], "qrcodeKey": data["data"]["qrcode_key"] }))
}

#[tauri::command(rename_all = "camelCase")]
async fn qr_poll(state: State<'_, AppState>, qrcodeKey: String) -> Result<Value, String> {
  let response = state.client.get(format!("{BILI_PASSPORT}/x/passport-login/web/qrcode/poll?qrcode_key={}", urlencoding::encode(&qrcodeKey)))
    .header("Referer", format!("{BILI_PASSPORT}/login")).send().await.map_err(|e| e.to_string())?;
  for value in response.headers().get_all("set-cookie").iter() {
    if let Ok(raw) = value.to_str() {
      if let Some((name, cookie)) = parse_cookie_pair(raw) { let _ = set_cookie(&state, name, cookie).await; }
    }
  }
  let status = response.status();
  if status == StatusCode::MOVED_PERMANENTLY || status == StatusCode::FOUND {
    return Ok(json!({ "code": 0, "status": 0, "message": "登录成功", "url": response.headers().get("location").and_then(|v| v.to_str().ok()).unwrap_or("") }));
  }
  let data: Value = response.json().await.map_err(|e| e.to_string())?;
  let code = data["data"]["code"].as_i64().or_else(|| data["code"].as_i64()).unwrap_or(-1);
  Ok(json!({ "code": code, "status": code, "message": data["data"]["message"].as_str().or_else(|| data["message"].as_str()).unwrap_or(""), "url": data["data"]["url"].as_str().unwrap_or("") }))
}

#[tauri::command]
async fn get_cookies(state: State<'_, AppState>) -> Result<CookiesInfo, String> {
  let cookies = cookie_header(&state).await;
  let sessdata = cookie_value(&cookies, "SESSDATA");
  let bili_jct = cookie_value(&cookies, "bili_jct");
  let dede_user_id = cookie_value(&cookies, "DedeUserID");
  Ok(CookiesInfo { is_logged_in: !sessdata.is_empty() && !bili_jct.is_empty() && !dede_user_id.is_empty(), sessdata, bili_jct, dede_user_id })
}

#[tauri::command]
async fn logout(state: State<'_, AppState>) -> Result<Value, String> {
  *state.cookies.lock().await = String::new();
  save_text(&app_file(&state, "bilibili-cookies.txt"), "")?;
  Ok(json!({ "success": true }))
}

fn webdav_url(base: &str, rel_path: &str) -> Result<String, String> {
  if rel_path.contains("..") || rel_path.contains('\\') { return Err("Invalid WebDAV path".to_string()) }
  Ok(format!("{}/bilimusic/{}", base.trim_end_matches('/'), rel_path.trim_start_matches('/')))
}

async fn dav_request(state: &AppState, cfg: &WebdavConfig, url: String, method: reqwest::Method, body: Option<String>, etag: Option<String>) -> Result<reqwest::Response, String> {
  let auth = format!("Basic {}", BASE64.encode(format!("{}:{}", cfg.username, cfg.password)));
  let mut req = state.client.request(method, url).header("Authorization", auth);
  if let Some(value) = body { req = req.header("Content-Type", "application/json; charset=utf-8").body(value); }
  if let Some(value) = etag { req = req.header("If-Match", value); }
  req.send().await.map_err(|e| e.to_string())
}

async fn webdav_get_inner(state: &AppState, cfg: &WebdavConfig, rel_path: &str) -> WebdavResult {
  let Ok(url) = webdav_url(&cfg.url, rel_path) else { return WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some("路径无效".to_string()) } };
  match dav_request(state, cfg, url, reqwest::Method::GET, None, None).await {
    Ok(response) if response.status() == StatusCode::NOT_FOUND => WebdavResult { ok: true, status: 404, etag: None, content: None, message: None },
    Ok(response) if response.status() == StatusCode::UNAUTHORIZED => WebdavResult { ok: false, status: 401, etag: None, content: None, message: Some("认证失败".to_string()) },
    Ok(response) if response.status().is_success() => { let etag = response.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_string); let content = response.text().await.ok(); WebdavResult { ok: true, status: 200, etag, content, message: None } },
    Ok(response) => WebdavResult { ok: false, status: response.status().as_u16(), etag: None, content: None, message: Some(format!("HTTP {}", response.status())) },
    Err(error) => WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some(error) },
  }
}

#[tauri::command(rename_all = "camelCase")]
async fn configure_webdav(state: State<'_, AppState>, url: String, username: String, password: String) -> Result<Value, String> {
  let old = state.webdav.lock().await.clone();
  let cfg = WebdavConfig { url: url.trim().to_string(), username, password: if password.is_empty() { old.map(|c| c.password).unwrap_or_default() } else { password } };
  save_text(&app_file(&state, "webdav.json"), &serde_json::to_string(&cfg).map_err(|e| e.to_string())?)?;
  *state.webdav.lock().await = Some(cfg);
  Ok(json!({ "ok": true }))
}

#[tauri::command]
async fn get_webdav_config(state: State<'_, AppState>) -> Result<WebdavConfigInfo, String> {
  Ok(match state.webdav.lock().await.clone() { Some(cfg) => WebdavConfigInfo { url: cfg.url, username: cfg.username, configured: true }, None => WebdavConfigInfo { url: String::new(), username: String::new(), configured: false } })
}

#[tauri::command]
async fn test_webdav(state: State<'_, AppState>) -> Result<Value, String> {
  let Some(cfg) = state.webdav.lock().await.clone() else { return Ok(json!({ "ok": false, "message": "未配置" })) };
  match dav_request(&state, &cfg, cfg.url.clone(), reqwest::Method::GET, None, None).await { Ok(response) if response.status() == StatusCode::UNAUTHORIZED => Ok(json!({ "ok": false, "message": "认证失败：账号或密码错误" })), Ok(_) => Ok(json!({ "ok": true, "message": "连接成功" })), Err(error) => Ok(json!({ "ok": false, "message": error })) }
}

#[tauri::command(rename_all = "camelCase")]
async fn webdav_get(state: State<'_, AppState>, relPath: String) -> Result<WebdavResult, String> {
  Ok(match state.webdav.lock().await.clone() { Some(cfg) => webdav_get_inner(&state, &cfg, &relPath).await, None => WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some("未配置".to_string()) } })
}

#[tauri::command(rename_all = "camelCase")]
async fn webdav_put(state: State<'_, AppState>, relPath: String, content: String, etag: Option<String>) -> Result<WebdavResult, String> {
  let Some(cfg) = state.webdav.lock().await.clone() else { return Ok(WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some("未配置".to_string()) }) };
  let Ok(url) = webdav_url(&cfg.url, &relPath) else { return Ok(WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some("路径无效".to_string()) }) };
  let _ = dav_request(&state, &cfg, format!("{}/", cfg.url.trim_end_matches('/')), reqwest::Method::from_bytes(b"MKCOL").unwrap(), None, None).await;
  Ok(match dav_request(&state, &cfg, url, reqwest::Method::PUT, Some(content), etag).await { Ok(response) if response.status() == StatusCode::PRECONDITION_FAILED => WebdavResult { ok: false, status: 412, etag: None, content: None, message: Some("远端已变更，需重新合并".to_string()) }, Ok(response) if response.status().is_success() => WebdavResult { ok: true, status: response.status().as_u16(), etag: response.headers().get("etag").and_then(|v| v.to_str().ok()).map(str::to_string), content: None, message: None }, Ok(response) => WebdavResult { ok: false, status: response.status().as_u16(), etag: None, content: None, message: Some(format!("HTTP {}", response.status())) }, Err(error) => WebdavResult { ok: false, status: 0, etag: None, content: None, message: Some(error) } })
}

#[tauri::command]
async fn clear_webdav(state: State<'_, AppState>) -> Result<Value, String> {
  *state.webdav.lock().await = None;
  let path = app_file(&state, "webdav.json");
  let _ = fs::remove_file(path);
  Ok(json!({ "ok": true }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrayState { has_track: bool, title: String, artist: String, is_playing: bool, queue_length: u32 }

#[tauri::command]
async fn update_tray_player_state(app: AppHandle, state: TrayState) -> Result<(), String> {
  if let Some(tray) = app.tray_by_id("main") { let _ = tray.set_tooltip(Some(if state.has_track { format!("{} - {}{}", state.title, state.artist, if state.is_playing { " ▶" } else { "" }) } else { "BiliMusic".to_string() })); }
  Ok(())
}

#[tauri::command]
async fn get_app_version(app: AppHandle) -> Result<String, String> { app.package_info().version.to_string().parse().map_err(|e| format!("{e:?}")) }

fn create_tray(app: &tauri::App) -> tauri::Result<()> {
  let icon = app
    .default_window_icon()
    .cloned()
    .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("Application icon is not configured")))?;
  let show = MenuItem::with_id(app, "show-window", "显示 BiliMusic", true, None::<&str>)?;
  let play = MenuItem::with_id(app, "toggle-play", "播放/暂停", true, None::<&str>)?;
  let prev = MenuItem::with_id(app, "prev", "上一首", true, None::<&str>)?;
  let next = MenuItem::with_id(app, "next", "下一首", true, None::<&str>)?;
  let quit = MenuItem::with_id(app, "quit", "退出应用", true, None::<&str>)?;
  let separator = PredefinedMenuItem::separator(app)?;
  let menu = Menu::with_items(app, &[&show, &play, &prev, &next, &separator, &quit])?;
  TrayIconBuilder::with_id("main").icon(icon).menu(&menu).tooltip("BiliMusic").on_menu_event(|app, event| {
    match event.id().as_ref() {
      "show-window" => { if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } }
      "quit" => app.exit(0),
      command => { let _ = app.emit("tray-player-command", command); }
    }
  }).build(app)?;
  Ok(())
}

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .setup(|app| {
      start_media_proxy().map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error)))?;
      let data_dir = app.path().app_data_dir()?;
      fs::create_dir_all(&data_dir)?;
      let cookies = load_text(&data_dir.join("bilibili-cookies.txt")).unwrap_or_default();
      let webdav = load_text(&data_dir.join("webdav.json")).and_then(|raw| serde_json::from_str::<WebdavConfig>(&raw).ok());
      let state = AppState {
        client: Client::builder()
          .redirect(reqwest::redirect::Policy::none())
          .build()
          .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!(e)))?,
        cookies: Arc::new(Mutex::new(cookies)),
        webdav: Arc::new(Mutex::new(webdav)),
        data_dir,
      };
      app.manage(state);
      create_tray(app)?;
      if let Some(window) = app.get_webview_window("main") {
        let close_window = window.clone();
        window.on_window_event(move |event| {
          if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = close_window.hide();
          }
        });
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      bili_raw, fetch_subtitle, download_audio, lyrics_search, lyrics_get,
      qr_generate, qr_poll, get_cookies, logout, configure_webdav,
      get_webdav_config, test_webdav, webdav_get, webdav_put, clear_webdav,
      update_tray_player_state, get_app_version,
    ])
    .run(tauri::generate_context!())
    .expect("error while running BiliMusic");
}
