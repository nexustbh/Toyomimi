// Qwen3.8-LiveTranslate-Flash realtime translation provider — backend WS bridge.
//
// Toyomimi: rewritten 2026-09-22 for qwen3.8 (protocol verified by
// scripts/phase0/probe.py against the cn-beijing endpoint; see docs/toyomimi/bugs.md B5).
//
// Key facts:
//   - WS URL: dashscope.aliyuncs.com (cn-beijing, default) or dashscope-intl
//     (ap-southeast-1) + ?model=qwen3.8-livetranslate-flash-realtime
//   - Audio in: pcm16 @ 16kHz mono
//   - Server-VAD only; the server segments turns on pauses.
//   - session.update is sent AFTER session.created, uses `output_modalities`
//     and MUST set audio.output.voice (default voice is rejected with 400).
//   - Source language: "auto"/empty → omitted (server auto-detects).
//
// Event mapping:
//   response.text.delta → Transcript(is_final=false, text=accumulated snapshot)
//   response.text.done  → Transcript(is_final=true,  text=final)
//   error               → Error

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::{SinkExt, StreamExt};
use http::Request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::State;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const QWEN_MODEL: &str = "qwen3.8-livetranslate-flash-realtime";
/// Output voice must be explicit even in text-only mode (server default is rejected).
const QWEN_VOICE: &str = "Tina";

fn qwen_host(region: Option<&str>) -> &'static str {
    match region {
        Some("ap-southeast-1") | Some("intl") => "dashscope-intl.aliyuncs.com",
        _ => "dashscope.aliyuncs.com", // cn-beijing (default)
    }
}

#[derive(Debug, Deserialize)]
pub struct QwenRealtimeConfig {
    pub api_key: String,
    /// BCP-47-ish code (e.g. "en", "ja"). "auto" or empty → server auto-detect.
    pub source_language: String,
    /// BCP-47-ish code (e.g. "vi"). Sent as `translation.language`.
    pub target_language: String,
    /// "cn-beijing" (default) or "ap-southeast-1".
    #[serde(default)]
    pub region: Option<String>,
    /// Optional DashScope workspace id (sent as X-DashScope-WorkSpace).
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QwenEvent {
    Status {
        state: String,
        message: Option<String>,
    },
    Transcript {
        text: String,
        is_final: bool,
    },
    Error {
        code: String,
        message: String,
    },
    Closed {
        reason: String,
    },
}

struct Session {
    audio_tx: mpsc::UnboundedSender<Vec<u8>>,
    stop_tx: mpsc::UnboundedSender<()>,
}

#[derive(Default)]
pub struct QwenState {
    sessions: Mutex<HashMap<u64, Session>>,
    next_id: Mutex<u64>,
}

#[tauri::command]
pub async fn qwen_realtime_start(
    config: QwenRealtimeConfig,
    on_event: Channel<QwenEvent>,
    state: State<'_, QwenState>,
) -> Result<u64, String> {
    if config.api_key.trim().is_empty() {
        return Err("Qwen (DashScope) API key is empty".into());
    }

    let session_id = {
        let mut id = state.next_id.lock().unwrap();
        *id += 1;
        *id
    };

    let (audio_tx, audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();

    let session = Session { audio_tx, stop_tx };
    state.sessions.lock().unwrap().insert(session_id, session);

    let event_ch = on_event.clone();

    tokio::spawn(async move {
        let _ = event_ch.send(QwenEvent::Status {
            state: "connecting".into(),
            message: None,
        });

        if let Err(e) = run_session(config, audio_rx, stop_rx, event_ch.clone()).await {
            let _ = event_ch.send(QwenEvent::Error {
                code: "session_failed".into(),
                message: e,
            });
        }

        let _ = event_ch.send(QwenEvent::Closed {
            reason: "session_ended".into(),
        });
    });

    Ok(session_id)
}

#[tauri::command]
pub async fn qwen_realtime_send_audio(
    session_id: u64,
    pcm: Vec<u8>,
    state: State<'_, QwenState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    session
        .audio_tx
        .send(pcm)
        .map_err(|e| format!("send audio failed: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn qwen_realtime_stop(
    session_id: u64,
    state: State<'_, QwenState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.remove(&session_id) {
        let _ = session.stop_tx.send(());
    }
    Ok(())
}

async fn run_session(
    cfg: QwenRealtimeConfig,
    mut audio_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    mut stop_rx: mpsc::UnboundedReceiver<()>,
    event_ch: Channel<QwenEvent>,
) -> Result<(), String> {
    let host = qwen_host(cfg.region.as_deref());
    let url = format!("wss://{}/api-ws/v1/realtime?model={}", host, QWEN_MODEL);
    let mut builder = Request::builder()
        .uri(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key.trim()))
        .header("Host", host)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        ;
    if let Some(ws_id) = cfg.workspace_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        builder = builder.header("X-DashScope-WorkSpace", ws_id);
    }
    let request = builder
        .body(())
        .map_err(|e| format!("build request: {}", e))?;

    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| format!("websocket connect: {}", e))?;

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    // qwen3.8: wait for session.created before configuring the session.
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(15), ws_stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                if text.contains("\"session.created\"") {
                    break;
                }
                if text.contains("\"error\"") {
                    handle_server_event(&text, &event_ch, &mut None, &mut HashMap::new());
                    return Err("server rejected session".into());
                }
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => return Err(format!("ws error: {}", e)),
            Ok(None) => return Err("connection closed before session.created".into()),
            Err(_) => return Err("timeout waiting for session.created".into()),
        }
    }

    let session_update = build_session_update(&cfg);
    ws_sink
        .send(Message::Text(session_update.into()))
        .await
        .map_err(|e| format!("send session.update: {}", e))?;

    let _ = event_ch.send(QwenEvent::Status {
        state: "ready".into(),
        message: None,
    });

    // Guard: response.text.done can fire twice for one response when both text
    // and audio_transcript streams complete. With modalities=["text"] this
    // shouldn't happen, but keep the guard — cheap insurance, matches mobile.
    let mut last_done_response_id: Option<String> = None;
    // Accumulated translation deltas per response_id (qwen3.8 streams deltas).
    let mut partials: HashMap<String, String> = HashMap::new();

    loop {
        tokio::select! {
            biased;

            _ = stop_rx.recv() => {
                let _ = ws_sink.send(Message::Close(None)).await;
                break;
            }

            Some(pcm) = audio_rx.recv() => {
                let b64 = B64.encode(&pcm);
                let evt = serde_json::json!({
                    "type": "input_audio_buffer.append",
                    "audio": b64,
                });
                if let Err(e) = ws_sink.send(Message::Text(evt.to_string().into())).await {
                    return Err(format!("send audio: {}", e));
                }
            }

            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_server_event(&text, &event_ch, &mut last_done_response_id, &mut partials);
                    }
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame
                            .map(|f| format!("{}: {}", f.code, f.reason))
                            .unwrap_or_else(|| "remote_close".into());
                        let _ = event_ch.send(QwenEvent::Closed { reason });
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(format!("ws error: {}", e)),
                    None => break,
                }
            }
        }
    }

    Ok(())
}

fn build_session_update(cfg: &QwenRealtimeConfig) -> String {
    let mut session = serde_json::json!({
        "output_modalities": ["text"],
        "translation": { "language": cfg.target_language },
        "audio": { "output": { "voice": QWEN_VOICE } },
    });
    let source = cfg.source_language.trim();
    if !source.is_empty() && source != "auto" {
        session["input_audio_transcription"] = serde_json::json!({ "language": source });
    }

    serde_json::json!({
        "type": "session.update",
        "session": session,
    })
    .to_string()
}

fn handle_server_event(
    text: &str,
    event_ch: &Channel<QwenEvent>,
    last_done_response_id: &mut Option<String>,
    partials: &mut HashMap<String, String>,
) {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let evt_type = match value.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return,
    };

    eprintln!("[qwen-livetranslate] event: {}", evt_type);

    match evt_type {
        "session.created" | "session.updated" | "response.created" | "response.done" => {}

        "response.text.delta" => {
            // qwen3.8 streams translation as deltas; UI expects a full snapshot.
            let rid = value
                .get("response_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let delta = value.get("delta").and_then(|v| v.as_str()).unwrap_or("");
            let acc = partials.entry(rid).or_default();
            acc.push_str(delta);
            if !acc.is_empty() {
                let _ = event_ch.send(QwenEvent::Transcript {
                    text: acc.clone(),
                    is_final: false,
                });
            }
        }

        "response.text.done" => {
            let response_id = value
                .get("response_id")
                .and_then(|v| v.as_str())
                .or_else(|| value.get("item_id").and_then(|v| v.as_str()))
                .map(|s| s.to_string());

            if let (Some(rid), Some(last)) = (response_id.as_ref(), last_done_response_id.as_ref())
            {
                if rid == last {
                    return;
                }
            }
            let accumulated = response_id.as_ref().and_then(|rid| partials.remove(rid));
            *last_done_response_id = response_id;

            let t = value
                .get("text")
                .and_then(|v| v.as_str())
                .filter(|t| !t.is_empty())
                .or(accumulated.as_deref());
            if let Some(t) = t {
                eprintln!(
                    "[qwen-livetranslate] DONE: {}",
                    &t.chars().take(120).collect::<String>()
                );
                let _ = event_ch.send(QwenEvent::Transcript {
                    text: t.into(),
                    is_final: true,
                });
            }
        }

        "error" => {
            let code = value
                .get("error")
                .and_then(|e| e.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let msg = value
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let _ = event_ch.send(QwenEvent::Error { code, message: msg });
        }

        "response.cancelled" | "response.failed" | "response.incomplete" => {
            if let Some(rid) = value.get("response_id").and_then(|v| v.as_str()) {
                partials.remove(rid);
            }
        }

        _ => {}
    }
}
