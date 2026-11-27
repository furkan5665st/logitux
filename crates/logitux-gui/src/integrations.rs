use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;

use serde_json::{json, Value};

use crate::input_synth;
#[allow(unused_imports)]
use crate::store::{
    CustomAction, CustomActionKind, DiscordAction, DiscordSettings, IntegrationAction, ObsAction,
    ObsSettings, SlobsAction,
};

#[cfg(target_os = "windows")]
fn open_discord_ipc() -> Option<std::fs::File> {
    for i in 0..10 {
        let pipe = format!(r"\\.\pipe\discord-ipc-{i}");
        if let Ok(f) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe)
        {
            return Some(f);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn discord_ipc_paths() -> Vec<std::path::PathBuf> {
    use std::env;
    use std::path::PathBuf;

    let mut bases = Vec::new();
    if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(&xdg);
        bases.push(p.clone());
        bases.push(p.join("app/com.discordapp.Discord"));
        bases.push(p.join("app/com.discordapp.DiscordCanary"));
        bases.push(p.join("app/com.discordapp.DiscordPTB"));
        bases.push(p.join("app/dev.vencord.Vesktop"));
        bases.push(p.join("app/org.equicord.Equibop"));
        bases.push(p.join("snap.discord"));
    }
    if let Ok(tmp) = env::var("TMPDIR") {
        if !tmp.is_empty() {
            bases.push(PathBuf::from(tmp));
        }
    }
    bases.push(PathBuf::from("/tmp"));

    let mut paths = Vec::new();
    for base in bases {
        for i in 0..10 {
            paths.push(base.join(format!("discord-ipc-{i}")));
        }
    }
    paths
}

#[cfg(not(target_os = "windows"))]
fn open_discord_ipc() -> Option<std::os::unix::net::UnixStream> {
    use std::os::unix::net::UnixStream;

    for p in discord_ipc_paths() {
        if let Ok(s) = UnixStream::connect(&p) {
            let _ = s.set_read_timeout(Some(Duration::from_millis(1800)));
            let _ = s.set_write_timeout(Some(Duration::from_millis(1200)));
            return Some(s);
        }
    }
    None
}

pub fn is_discord_detected() -> bool {
    #[cfg(target_os = "windows")]
    {
        for i in 0..10 {
            let pipe = format!(r"\\.\pipe\discord-ipc-{i}");
            if std::fs::metadata(&pipe).is_ok() {
                return true;
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        discord_ipc_paths()
            .into_iter()
            .any(|p| std::fs::metadata(&p).is_ok())
    }
}

pub fn is_obs_detected(settings: &ObsSettings) -> bool {
    let addr_str = format!("{}:{}", settings.host, settings.port);
    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
        TcpStream::connect_timeout(&addr, Duration::from_millis(80)).is_ok()
    } else {
        false
    }
}

fn ipc_write_frame<W: Write>(w: &mut W, opcode: u32, payload: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let mut hdr = [0u8; 8];
    hdr[0..4].copy_from_slice(&opcode.to_le_bytes());
    hdr[4..8].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    w.write_all(&hdr).map_err(|e| e.to_string())?;
    w.write_all(&bytes).map_err(|e| e.to_string())?;
    w.flush().map_err(|e| e.to_string())
}

fn ipc_read_frame<R: Read>(r: &mut R) -> Result<(u32, Value), String> {
    let mut hdr = [0u8; 8];
    r.read_exact(&mut hdr).map_err(|e| e.to_string())?;
    let op = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]);
    let len = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    if len > 2 * 1024 * 1024 {
        return Err("Discord IPC frame too large".into());
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;
    Ok((op, v))
}

pub fn discord_test_or_authorize(client_id: &str, access_token: &str) -> Result<String, String> {
    let mut sock = open_discord_ipc().ok_or_else(|| {
        "Discord masaüstü uygulaması açık değil (discord-ipc bulunamadı).".to_string()
    })?;

    let cid = if client_id.trim().is_empty() {
        "227491271223017472"
    } else {
        client_id.trim()
    };

    ipc_write_frame(&mut sock, 0, &json!({ "v": 1, "client_id": cid }))?;
    let (_, ready) = ipc_read_frame(&mut sock)?;

    let ready_user = ready
        .pointer("/data/user/username")
        .and_then(|v| v.as_str())
        .unwrap_or("Discord Kullanıcısı")
        .to_string();

    if !access_token.trim().is_empty() {
        ipc_write_frame(
            &mut sock,
            1,
            &json!({
                "cmd": "AUTHENTICATE",
                "args": { "access_token": access_token.trim() },
                "nonce": "logitux-auth-1"
            }),
        )?;
        let (_, auth_res) = ipc_read_frame(&mut sock)?;
        if auth_res.get("evt").and_then(|e| e.as_str()) == Some("ERROR") {
            let msg = auth_res
                .pointer("/data/message")
                .and_then(|m| m.as_str())
                .unwrap_or("Token geçersiz");
            return Err(format!("Discord RPC Auth hatası: {msg}"));
        }
        return Ok(format!("{ready_user} (RPC Tam Yetkili)"));
    }

    let _ = ipc_write_frame(&mut sock, 2, &json!({}));
    Ok(format!("{ready_user} (Algılandı · Kısayol Modu)"))
}

pub fn execute_discord_action(action: &DiscordAction, cfg: &DiscordSettings) -> Result<(), String> {
    if cfg.prefer_rpc && !cfg.access_token.trim().is_empty() {
        if let Some(mut sock) = open_discord_ipc() {
            let cid = if cfg.client_id.trim().is_empty() {
                "227491271223017472"
            } else {
                cfg.client_id.trim()
            };
            ipc_write_frame(&mut sock, 0, &json!({ "v": 1, "client_id": cid }))?;
            let _ = ipc_read_frame(&mut sock)?;
            ipc_write_frame(
                &mut sock,
                1,
                &json!({
                    "cmd": "AUTHENTICATE",
                    "args": { "access_token": cfg.access_token.trim() },
                    "nonce": "logitux-exec-auth"
                }),
            )?;
            let _ = ipc_read_frame(&mut sock)?;

            match action {
                DiscordAction::ToggleMute | DiscordAction::ToggleDeafen => {
                    ipc_write_frame(
                        &mut sock,
                        1,
                        &json!({
                            "cmd": "GET_VOICE_SETTINGS",
                            "args": {},
                            "nonce": "logitux-get-vs"
                        }),
                    )?;
                    let (_, vs) = ipc_read_frame(&mut sock)?;
                    let cur_mute = vs
                        .pointer("/data/mute")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(false);
                    let cur_deaf = vs
                        .pointer("/data/deaf")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(false);
                    let args = if matches!(action, DiscordAction::ToggleMute) {
                        json!({ "mute": !cur_mute })
                    } else {
                        json!({ "deaf": !cur_deaf })
                    };
                    ipc_write_frame(
                        &mut sock,
                        1,
                        &json!({
                            "cmd": "SET_VOICE_SETTINGS",
                            "args": args,
                            "nonce": "logitux-set-vs"
                        }),
                    )?;
                    let _ = ipc_read_frame(&mut sock);
                    return Ok(());
                }
                DiscordAction::JoinVoice { channel_id, .. } => {
                    ipc_write_frame(
                        &mut sock,
                        1,
                        &json!({
                            "cmd": "SELECT_VOICE_CHANNEL",
                            "args": { "channel_id": channel_id, "force": true },
                            "nonce": "logitux-join-vc"
                        }),
                    )?;
                    let _ = ipc_read_frame(&mut sock);
                    return Ok(());
                }
                DiscordAction::LeaveVoice => {
                    ipc_write_frame(
                        &mut sock,
                        1,
                        &json!({
                            "cmd": "SELECT_VOICE_CHANNEL",
                            "args": { "channel_id": Value::Null, "force": true },
                            "nonce": "logitux-leave-vc"
                        }),
                    )?;
                    let _ = ipc_read_frame(&mut sock);
                    return Ok(());
                }
                DiscordAction::PushToTalk => {}
            }
        }
    }

    match action {
        DiscordAction::ToggleMute => {
            input_synth::tap_shortcut(cfg.mute_shortcut.0, cfg.mute_shortcut.1);
            Ok(())
        }
        DiscordAction::ToggleDeafen => {
            input_synth::tap_shortcut(cfg.deafen_shortcut.0, cfg.deafen_shortcut.1);
            Ok(())
        }
        DiscordAction::PushToTalk => {
            input_synth::tap_shortcut(cfg.ptt_shortcut.0, cfg.ptt_shortcut.1);
            Ok(())
        }
        DiscordAction::JoinVoice { .. } | DiscordAction::LeaveVoice => {
            Err("Ses kanalı değiştirme için Discord RPC Erişim Anahtarı (Token) gereklidir.".into())
        }
    }
}

fn ws_send_text(stream: &mut TcpStream, text: &str) -> Result<(), String> {
    let payload = text.as_bytes();
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x81);
    let mask_key: [u8; 4] = [0x37, 0xFA, 0x21, 0x3D];
    if payload.len() < 126 {
        frame.push(0x80 | (payload.len() as u8));
    } else if payload.len() <= 65535 {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        return Err("WS frame too large".into());
    }
    frame.extend_from_slice(&mask_key);
    for (i, &b) in payload.iter().enumerate() {
        frame.push(b ^ mask_key[i & 3]);
    }
    stream.write_all(&frame).map_err(|e| e.to_string())
}

fn ws_read_text(stream: &mut TcpStream) -> Result<String, String> {
    let mut hdr = [0u8; 2];
    stream.read_exact(&mut hdr).map_err(|e| e.to_string())?;
    let masked = (hdr[1] & 0x80) != 0;
    let mut len = (hdr[1] & 0x7F) as usize;
    if len == 126 {
        let mut ext = [0u8; 2];
        stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
        len = u16::from_be_bytes(ext) as usize;
    } else if len == 127 {
        let mut ext = [0u8; 8];
        stream.read_exact(&mut ext).map_err(|e| e.to_string())?;
        len = u64::from_be_bytes(ext) as usize;
    }
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;
    if masked {
        for (i, b) in buf.iter_mut().enumerate() {
            *b ^= mask[i & 3];
        }
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub fn execute_obs_action(action: ObsAction, settings: &ObsSettings) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{}", settings.host, settings.port)
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(450))
        .map_err(|_| "OBS WebSocket (4455) bağlı değil.".to_string())?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));

    let handshake = format!(
        "GET / HTTP/1.1\r\nHost: {}:{}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        settings.host, settings.port
    );
    stream
        .write_all(handshake.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut resp_buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    while resp_buf.len() < 4096 {
        stream.read_exact(&mut byte).map_err(|e| e.to_string())?;
        resp_buf.push(byte[0]);
        if resp_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let _hello = ws_read_text(&mut stream)?;

    ws_send_text(
        &mut stream,
        &json!({
            "op": 1,
            "d": { "rpcVersion": 1 }
        })
        .to_string(),
    )?;
    let _identified = ws_read_text(&mut stream)?;

    let (req_type, req_data) = match action {
        ObsAction::StartStream => ("StartStream", json!({})),
        ObsAction::StopStream => ("StopStream", json!({})),
        ObsAction::ToggleStream => ("ToggleStream", json!({})),
        ObsAction::StartRecording => ("StartRecord", json!({})),
        ObsAction::StopRecording => ("StopRecord", json!({})),
        ObsAction::ToggleRecording => ("ToggleRecord", json!({})),
        ObsAction::StartBuffer => ("StartReplayBuffer", json!({})),
        ObsAction::StopBuffer => ("StopReplayBuffer", json!({})),
        ObsAction::ToggleBuffer => ("ToggleReplayBuffer", json!({})),
        ObsAction::SaveBuffer => ("SaveReplayBuffer", json!({})),
        ObsAction::DesktopMute => (
            "SetInputMute",
            json!({ "inputName": "Desktop Audio", "inputMuted": true }),
        ),
        ObsAction::DesktopUnmute => (
            "SetInputMute",
            json!({ "inputName": "Desktop Audio", "inputMuted": false }),
        ),
        ObsAction::DesktopMuteToggle => {
            ("ToggleInputMute", json!({ "inputName": "Desktop Audio" }))
        }
        ObsAction::MicMute => (
            "SetInputMute",
            json!({ "inputName": "Mic/Aux", "inputMuted": true }),
        ),
        ObsAction::MicUnmute => (
            "SetInputMute",
            json!({ "inputName": "Mic/Aux", "inputMuted": false }),
        ),
        ObsAction::MicMuteToggle => ("ToggleInputMute", json!({ "inputName": "Mic/Aux" })),
    };

    ws_send_text(
        &mut stream,
        &json!({
            "op": 6,
            "d": {
                "requestType": req_type,
                "requestId": "logitux-obs-1",
                "requestData": req_data
            }
        })
        .to_string(),
    )?;
    let _ = ws_read_text(&mut stream);
    Ok(())
}

pub fn execute_custom_action(act: &CustomAction) -> Result<(), String> {
    match &act.kind {
        CustomActionKind::LaunchApp { path, args } => {
            let mut cmd = Command::new(path);
            if !args.trim().is_empty() {
                cmd.args(args.split_whitespace());
            }
            cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
        }
        CustomActionKind::OpenUrl { url } => {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(["/C", "start", "", url])
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("xdg-open")
                    .arg(url)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        }
        CustomActionKind::ShellCmd { command } => {
            #[cfg(target_os = "windows")]
            {
                Command::new("powershell")
                    .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", command])
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(target_os = "windows"))]
            {
                Command::new("sh")
                    .args(["-c", command])
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        }
    }
}

pub fn execute_integration(
    action: &IntegrationAction,
    discord_cfg: &DiscordSettings,
    obs_cfg: &ObsSettings,
) -> Result<(), String> {
    match action {
        IntegrationAction::Discord(d) => execute_discord_action(d, discord_cfg),
        IntegrationAction::Obs(o) => execute_obs_action(*o, obs_cfg),
        IntegrationAction::Slobs(s) => {
            #[cfg(target_os = "windows")]
            {
                if let Ok(mut pipe) = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(r"\\.\pipe\slobs")
                {
                    let method = match s {
                        SlobsAction::ToggleStream => "StreamingService.toggleStreaming",
                        SlobsAction::ToggleRecording => "StreamingService.toggleRecording",
                        SlobsAction::ActivateScene(_) => "ScenesService.makeSceneActive",
                        SlobsAction::ToggleSource(_) => "SourcesService.setMuted",
                        SlobsAction::MixerMuteToggle => "AudioService.setMuted",
                    };
                    let msg = format!(
                        "{}\n",
                        json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": method,
                            "params": { "resource": method.split('.').next().unwrap_or("") }
                        })
                    );
                    let _ = pipe.write_all(msg.as_bytes());
                    return Ok(());
                }
            }
            let _ = s;
            Err("Streamlabs Desktop açık değil.".into())
        }
        IntegrationAction::Overwolf { command } => {
            let _ = command;
            Err("Overwolf uzantısı bağlı değil.".into())
        }
        IntegrationAction::Custom(c) => execute_custom_action(c),
    }
}
