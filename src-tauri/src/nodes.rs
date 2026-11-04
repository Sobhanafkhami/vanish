//! Share-link / subscription parsing. Every node is stored as a ready-to-use
//! sing-box outbound object so config generation is a plain JSON merge.

use std::collections::HashMap;

use base64::{
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub server: String,
    pub port: u16,
    pub outbound: Value,
    pub latency: Option<u32>,
    /// Id of the owning subscription; `None` for manually added nodes.
    pub subscription: Option<String>,
    /// Original share link (when imported from one), used for QR export.
    #[serde(default)]
    pub link: Option<String>,
    /// Lifetime bytes sent / received through this node.
    #[serde(default)]
    pub up_total: u64,
    #[serde(default)]
    pub down_total: u64,
    /// Real HTTP delay in ms; -1 = test failed, None = never tested.
    #[serde(default)]
    pub delay: Option<i32>,
    /// Xray outbound for links whose transport sing-box cannot dial (XHTTP, mKCP, TCP HTTP-header, VLESS
    /// encryption). `outbound` stays the closest sing-box equivalent for display; connections go through Xray.
    #[serde(default)]
    pub xray: Option<Value>,
}

type Query = HashMap<String, String>;

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
        .iter()
        .find_map(|e| e.decode(s).ok())
}

fn pct(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().into_owned()
}

fn node_from_outbound(name: String, outbound: Value) -> Result<Node, String> {
    let protocol = outbound["type"].as_str().ok_or("outbound has no type")?.to_string();
    let server = outbound["server"].as_str().ok_or("outbound has no server")?.to_string();
    let port = outbound["server_port"].as_u64().ok_or("outbound has no server_port")? as u16;
    Ok(Node {
        id: uuid::Uuid::new_v4().to_string(),
        name: if name.is_empty() { format!("{server}:{port}") } else { name },
        protocol,
        server,
        port,
        outbound,
        latency: None,
        subscription: None,
        link: None,
        up_total: 0,
        down_total: 0,
        delay: None,
        xray: None,
    })
}

fn nonempty<'a>(q: &'a Query, key: &str) -> Option<&'a str> {
    q.get(key).map(String::as_str).filter(|s| !s.is_empty())
}

fn net_type(q: &Query) -> &str {
    match nonempty(q, "type").unwrap_or("tcp") {
        "raw" => "tcp",
        "mkcp" => "kcp",
        "splithttp" => "xhttp",
        "h2" => "http",
        other => other,
    }
}

fn tls_from(q: &Query, server: &str, default_tls: bool) -> Option<Value> {
    let sec = nonempty(q, "security").unwrap_or(if default_tls { "tls" } else { "none" });
    if sec == "none" {
        return None;
    }
    // An empty `sni=` (common in vmess:// links) must fall back to the Host header, then the server, like v2rayNG.
    let sni = nonempty(q, "sni").or(nonempty(q, "peer")).or(nonempty(q, "host")).unwrap_or(server);
    let mut tls = json!({ "enabled": true, "server_name": sni });
    if matches!(q.get("allowInsecure").or(q.get("insecure")).map(String::as_str), Some("1" | "true")) {
        tls["insecure"] = json!(true);
    }
    // Without an explicit ALPN a Chrome-like hello offers h2, a CDN picks it, and a WebSocket upgrade then
    // never completes (a timeout in the delay test). Xray-based clients pin the ALPN per transport.
    let alpn: Vec<&str> = match nonempty(q, "alpn") {
        Some(a) => a.split(',').map(str::trim).filter(|s| !s.is_empty()).collect(),
        None => match net_type(q) {
            "ws" | "httpupgrade" => vec!["http/1.1"],
            "grpc" | "http" => vec!["h2"],
            _ => vec![],
        },
    };
    if !alpn.is_empty() {
        tls["alpn"] = json!(alpn);
    }
    if let Some(fp) = nonempty(q, "fp") {
        tls["utls"] = json!({ "enabled": true, "fingerprint": fp });
    }
    if sec == "reality" {
        tls["reality"] = json!({
            "enabled": true,
            "public_key": q.get("pbk").cloned().unwrap_or_default(),
            "short_id": q.get("sid").cloned().unwrap_or_default(),
        });
        if tls.get("utls").is_none() {
            tls["utls"] = json!({ "enabled": true, "fingerprint": "chrome" });
        }
    }
    Some(tls)
}

fn transport_from(q: &Query) -> Option<Value> {
    let path = nonempty(q, "path").unwrap_or("/").to_string();
    // Host header: explicit, else the TLS server name (what v2rayNG sends), else leave it to the dialer.
    let host = nonempty(q, "host").or(nonempty(q, "sni")).map(str::to_string);
    match net_type(q) {
        "ws" => {
            let mut v = json!({ "type": "ws", "path": path });
            if let Some(h) = host {
                v["headers"] = json!({ "Host": h });
            }
            Some(v)
        }
        "grpc" => Some(json!({
            "type": "grpc",
            "service_name": q.get("serviceName").cloned().unwrap_or_default(),
        })),
        "http" => {
            let mut v = json!({ "type": "http", "path": path });
            if let Some(h) = host {
                v["host"] = json!(h.split(',').collect::<Vec<_>>());
            }
            Some(v)
        }
        "httpupgrade" => {
            let mut v = json!({ "type": "httpupgrade", "path": path });
            if let Some(h) = host {
                v["host"] = json!(h);
            }
            Some(v)
        }
        "quic" => Some(json!({ "type": "quic" })),
        _ => None,
    }
}

/// True when sing-box has no client for what the link asks for and Xray must carry it.
fn needs_xray(q: &Query) -> bool {
    let encrypted = nonempty(q, "encryption").is_some_and(|e| e != "none");
    matches!(net_type(q), "xhttp" | "kcp") || (net_type(q) == "tcp" && nonempty(q, "headerType") == Some("http")) || encrypted
}

/// Xray `streamSettings` for a link's query.
fn xray_stream(q: &Query, server: &str) -> Value {
    let net = net_type(q);
    let sec = nonempty(q, "security").unwrap_or("none");
    let host = nonempty(q, "host");
    let path = nonempty(q, "path").unwrap_or("/");
    let mut s = json!({ "network": net, "security": sec });
    let sni = nonempty(q, "sni").or(nonempty(q, "peer")).or(host).unwrap_or(server);
    let alpn: Vec<&str> = nonempty(q, "alpn").map(|a| a.split(',').map(str::trim).filter(|x| !x.is_empty()).collect()).unwrap_or_default();
    match sec {
        "tls" => {
            let mut t = json!({ "serverName": sni });
            if !alpn.is_empty() {
                t["alpn"] = json!(alpn);
            }
            if let Some(fp) = nonempty(q, "fp") {
                t["fingerprint"] = json!(fp);
            }
            if matches!(q.get("allowInsecure").or(q.get("insecure")).map(String::as_str), Some("1" | "true")) {
                t["allowInsecure"] = json!(true);
            }
            s["tlsSettings"] = t;
        }
        "reality" => {
            s["realitySettings"] = json!({
                "serverName": sni,
                "fingerprint": nonempty(q, "fp").unwrap_or("chrome"),
                "publicKey": q.get("pbk").cloned().unwrap_or_default(),
                "shortId": q.get("sid").cloned().unwrap_or_default(),
                "spiderX": q.get("spx").cloned().unwrap_or_default(),
            });
        }
        _ => {}
    }
    match net {
        "xhttp" => {
            let mut x = json!({ "path": path, "host": host.unwrap_or(sni) });
            if let Some(m) = nonempty(q, "mode") {
                x["mode"] = json!(m);
            }
            if let Some(extra) = nonempty(q, "extra").and_then(|e| serde_json::from_str::<Value>(e).ok()) {
                x["extra"] = extra;
            }
            s["xhttpSettings"] = x;
        }
        "kcp" => {
            let mut k = json!({ "header": { "type": nonempty(q, "headerType").unwrap_or("none") } });
            if let Some(seed) = nonempty(q, "seed") {
                k["seed"] = json!(seed);
            }
            s["kcpSettings"] = k;
        }
        "tcp" if nonempty(q, "headerType") == Some("http") => {
            let hosts: Vec<&str> = host.map(|h| h.split(',').collect()).unwrap_or_default();
            s["tcpSettings"] = json!({ "header": { "type": "http", "request": {
                "path": path.split(',').collect::<Vec<_>>(),
                "headers": { "Host": hosts },
            } } });
        }
        "ws" => {
            s["wsSettings"] = json!({ "path": path, "host": host.unwrap_or(sni) });
        }
        "grpc" => {
            s["grpcSettings"] = json!({ "serviceName": q.get("serviceName").cloned().unwrap_or_default() });
        }
        "httpupgrade" => {
            s["httpupgradeSettings"] = json!({ "path": path, "host": host.unwrap_or(sni) });
        }
        _ => {}
    }
    s
}

/// Xray outbound for `proto` (`vless` / `vmess` / `trojan`); `user` is the protocol-specific account object.
fn xray_outbound(proto: &str, server: &str, port: u16, user: Value, q: &Query) -> Value {
    let settings = match proto {
        "trojan" => json!({ "servers": [{ "address": server, "port": port, "password": user["password"] }] }),
        _ => json!({ "vnext": [{ "address": server, "port": port, "users": [user] }] }),
    };
    json!({ "protocol": proto, "settings": settings, "streamSettings": xray_stream(q, server) })
}

fn split_url(link: &str) -> Result<(Url, String, u16, Query, String), String> {
    let url = Url::parse(link).map_err(|e| format!("bad link: {e}"))?;
    let host = url.host_str().ok_or("missing host")?.trim_matches(['[', ']']).to_string();
    let port = url.port().ok_or("missing port")?;
    let q: Query = url.query_pairs().into_owned().collect();
    let name = url.fragment().map(pct).unwrap_or_default();
    Ok((url, host, port, q, name))
}

fn parse_vless(link: &str) -> Result<Node, String> {
    let (url, host, port, q, name) = split_url(link)?;
    let mut ob = json!({
        "type": "vless", "server": host, "server_port": port, "uuid": pct(url.username()),
    });
    if let Some(flow) = q.get("flow").filter(|s| !s.is_empty()) {
        ob["flow"] = json!(flow);
    }
    if let Some(tls) = tls_from(&q, &host, false) {
        ob["tls"] = tls;
    }
    if let Some(t) = transport_from(&q) {
        ob["transport"] = t;
    }
    let xray = needs_xray(&q).then(|| {
        let mut user = json!({ "id": pct(url.username()), "encryption": nonempty(&q, "encryption").unwrap_or("none") });
        if let Some(flow) = nonempty(&q, "flow") {
            user["flow"] = json!(flow);
        }
        xray_outbound("vless", &host, port, user, &q)
    });
    node_from_outbound(name, ob).map(|n| Node { xray, ..n })
}

fn parse_trojan(link: &str) -> Result<Node, String> {
    let (url, host, port, q, name) = split_url(link)?;
    let mut ob = json!({
        "type": "trojan", "server": host, "server_port": port, "password": pct(url.username()),
    });
    if let Some(tls) = tls_from(&q, &host, true) {
        ob["tls"] = tls;
    }
    if let Some(t) = transport_from(&q) {
        ob["transport"] = t;
    }
    let xray = needs_xray(&q).then(|| xray_outbound("trojan", &host, port, json!({ "password": pct(url.username()) }), &q));
    node_from_outbound(name, ob).map(|n| Node { xray, ..n })
}

fn parse_vmess(link: &str) -> Result<Node, String> {
    let raw = b64_decode(link.trim_start_matches("vmess://")).ok_or("vmess: invalid base64")?;
    let v: Value = serde_json::from_slice(&raw).map_err(|e| format!("vmess: {e}"))?;
    let s = |k: &str| v[k].as_str().map(str::to_string).or_else(|| v[k].as_u64().map(|n| n.to_string())).unwrap_or_default();
    let server = s("add");
    let port: u16 = s("port").parse().map_err(|_| "vmess: bad port")?;

    let mut q = Query::new();
    q.insert("type".into(), s("net"));
    q.insert("host".into(), s("host"));
    q.insert("path".into(), s("path"));
    q.insert("serviceName".into(), s("path"));
    q.insert("security".into(), if s("tls") == "tls" { "tls".into() } else { "none".into() });
    q.insert("sni".into(), s("sni"));
    q.insert("alpn".into(), s("alpn"));
    q.insert("fp".into(), s("fp"));
    q.insert("headerType".into(), s("type"));
    q.insert("mode".into(), s("mode"));
    q.insert("extra".into(), s("extra"));

    let mut ob = json!({
        "type": "vmess", "server": server, "server_port": port, "uuid": s("id"),
        "security": if s("scy").is_empty() { "auto".to_string() } else { s("scy") },
        "alter_id": s("aid").parse::<u32>().unwrap_or(0),
    });
    if let Some(tls) = tls_from(&q, &server, false) {
        ob["tls"] = tls;
    }
    if let Some(t) = transport_from(&q) {
        ob["transport"] = t;
    }
    let xray = needs_xray(&q).then(|| {
        let user = json!({ "id": s("id"), "alterId": s("aid").parse::<u32>().unwrap_or(0),
            "security": if s("scy").is_empty() { "auto".to_string() } else { s("scy") } });
        xray_outbound("vmess", &server, port, user, &q)
    });
    node_from_outbound(s("ps"), ob).map(|n| Node { xray, ..n })
}

fn parse_ss(link: &str) -> Result<Node, String> {
    let (url, host, port, _q, name) = split_url(link).or_else(|_| {
        // Legacy form: ss://BASE64(method:pass@host:port)#name
        let body = link.trim_start_matches("ss://");
        let (b, frag) = body.split_once('#').unwrap_or((body, ""));
        let dec = String::from_utf8(b64_decode(b).ok_or("ss: invalid base64")?).map_err(|e| e.to_string())?;
        Url::parse(&format!("ss://{dec}#{frag}")).map_err(|e| e.to_string()).and_then(|u| {
            let host = u.host_str().ok_or("ss: missing host")?.to_string();
            let port = u.port().ok_or("ss: missing port")?;
            Ok((u.clone(), host, port, Query::new(), pct(frag)))
        })
    })?;
    let (method, password) = if let Some(p) = url.password() {
        (pct(url.username()), pct(p))
    } else {
        let dec = String::from_utf8(b64_decode(&pct(url.username())).ok_or("ss: invalid userinfo")?).map_err(|e| e.to_string())?;
        let (m, p) = dec.split_once(':').ok_or("ss: bad userinfo")?;
        (m.to_string(), p.to_string())
    };
    node_from_outbound(
        name,
        json!({ "type": "shadowsocks", "server": host, "server_port": port, "method": method, "password": password }),
    )
}

/// `wireguard://PRIVATE_KEY@host:port?publickey=..&address=10.0.0.2/32[,fd00::2/128]&mtu=1420#name`
fn parse_wireguard(link: &str) -> Result<Node, String> {
    let (url, host, port, q, name) = split_url(link)?;
    let addrs: Vec<&str> = q.get("address").map(|a| a.split(',').collect()).unwrap_or_default();
    if addrs.is_empty() {
        return Err("wireguard: missing address".into());
    }
    let mut ob = json!({
        "type": "wireguard", "server": host, "server_port": port,
        "local_address": addrs,
        "private_key": pct(url.username()),
        "peer_public_key": q.get("publickey").or(q.get("peer_public_key")).cloned().ok_or("wireguard: missing publickey")?,
    });
    if let Some(mtu) = q.get("mtu").and_then(|m| m.parse::<u32>().ok()) {
        ob["mtu"] = json!(mtu);
    }
    if let Some(psk) = q.get("presharedkey") {
        ob["pre_shared_key"] = json!(psk);
    }
    node_from_outbound(name, ob)
}

fn parse_json_outbounds(text: &str) -> Vec<Result<Node, String>> {
    let Ok(v) = serde_json::from_str::<Value>(text) else { return vec![] };
    let items: Vec<Value> = match v {
        Value::Array(a) => a,
        Value::Object(ref o) if o.contains_key("outbounds") => v["outbounds"].as_array().cloned().unwrap_or_default(),
        Value::Object(_) => vec![v],
        _ => vec![],
    };
    items
        .into_iter()
        .filter(|o| {
            !matches!(
                o["type"].as_str(),
                None | Some("direct" | "block" | "dns" | "selector" | "urltest" | "shadowtls")
            ) && o.get("server").is_some()
        })
        .map(|mut o| {
            let name = o["tag"].as_str().unwrap_or_default().to_string();
            if let Some(m) = o.as_object_mut() {
                m.remove("tag");
            }
            node_from_outbound(name, o)
        })
        .collect()
}

pub fn parse_link(line: &str) -> Result<Node, String> {
    let line = line.trim();
    let mut node = match line.split_once("://").map(|(s, _)| s.to_ascii_lowercase()).as_deref() {
        Some("vless") => parse_vless(line),
        Some("vmess") => parse_vmess(line),
        Some("trojan") => parse_trojan(line),
        Some("ss") => parse_ss(line),
        Some("wireguard" | "wg") => parse_wireguard(line),
        _ => Err(format!("unsupported link: {}", line.chars().take(24).collect::<String>())),
    }?;
    node.link = Some(line.to_string());
    Ok(node)
}

/// Parses pasted text: single links, many lines, a base64 subscription body,
/// or raw sing-box outbound JSON (the way to import ShadowTLS-chained configs).
pub fn parse_text(text: &str) -> (Vec<Node>, Vec<String>) {
    let text = text.trim();
    let mut results = parse_json_outbounds(text);
    if results.is_empty() {
        let body = if text.contains("://") {
            text.to_string()
        } else {
            b64_decode(text).and_then(|b| String::from_utf8(b).ok()).unwrap_or_else(|| text.to_string())
        };
        results = body.lines().map(str::trim).filter(|l| !l.is_empty()).map(parse_link).collect();
    }
    let (mut ok, mut err) = (vec![], vec![]);
    for r in results {
        match r {
            Ok(n) => ok.push(n),
            Err(e) => err.push(e),
        }
    }
    (ok, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vless_reality() {
        let n = parse_link("vless://u-u-i-d@example.com:443?security=reality&sni=a.com&pbk=KEY&sid=ab&fp=chrome&type=grpc&serviceName=svc&flow=xtls-rprx-vision#My%20Node").unwrap();
        assert_eq!(n.name, "My Node");
        assert_eq!(n.outbound["tls"]["reality"]["public_key"], "KEY");
        assert_eq!(n.outbound["transport"]["service_name"], "svc");
    }

    #[test]
    fn vmess_ws() {
        let j = r#"{"v":"2","ps":"vm","add":"h.com","port":"443","id":"id1","aid":"0","net":"ws","host":"h.com","path":"/p","tls":"tls"}"#;
        let n = parse_link(&format!("vmess://{}", STANDARD.encode(j))).unwrap();
        assert_eq!(n.port, 443);
        assert_eq!(n.outbound["transport"]["path"], "/p");
    }

    #[test]
    fn ss_sip002() {
        let ui = URL_SAFE_NO_PAD.encode("aes-256-gcm:pw");
        let n = parse_link(&format!("ss://{ui}@1.2.3.4:8388#ss")).unwrap();
        assert_eq!(n.outbound["method"], "aes-256-gcm");
    }

    #[test]
    fn subscription_base64() {
        let body = STANDARD.encode("trojan://pw@t.com:443#a\ntrojan://pw2@t2.com:443#b\n");
        let (ok, err) = parse_text(&body);
        assert_eq!((ok.len(), err.len()), (2, 0));
    }

    #[test]
    fn subscription_base64_crlf_with_encoded_names() {
        let link = "vless://83baf1d4-f357-4baf-872f-d60f6c2c14fa@185.143.235.200:443?path=%2F&flow=&type=ws&host=a.example.com&security=tls&sni=a.example.com&allowInsecure=0&fp=chrome#%5BID%3A1%5D%20%F0%9F%93%8A%5B6GB%5D";
        let body = STANDARD.encode(format!("{link}\r\n{link}\r\n"));
        let (ok, err) = parse_text(&body);
        assert_eq!((ok.len(), err.len()), (2, 0), "{err:?}");
        assert_eq!(ok[0].server, "185.143.235.200");
    }

    #[test]
    fn xhttp_goes_to_xray() {
        let n = parse_link("vless://u-u-i-d@example.com:443?security=tls&type=xhttp&path=%2Fx&host=h.example.com&mode=packet-up&sni=h.example.com&fp=chrome&extra=%7B%22xPaddingBytes%22%3A%22100-1000%22%7D#x").unwrap();
        let x = n.xray.expect("xhttp needs xray");
        assert_eq!(x["streamSettings"]["network"], "xhttp");
        assert_eq!(x["streamSettings"]["xhttpSettings"]["mode"], "packet-up");
        assert_eq!(x["streamSettings"]["xhttpSettings"]["extra"]["xPaddingBytes"], "100-1000");
        assert_eq!(x["streamSettings"]["tlsSettings"]["serverName"], "h.example.com");
        assert_eq!(x["settings"]["vnext"][0]["users"][0]["id"], "u-u-i-d");
    }

    #[test]
    fn ws_pins_alpn_and_falls_back_to_host_for_sni() {
        let j = r#"{"v":"2","ps":"vm","add":"1.2.3.4","port":"443","id":"id1","aid":"0","net":"ws","host":"cdn.example.com","path":"/p","tls":"tls","sni":"","fp":"chrome"}"#;
        let n = parse_link(&format!("vmess://{}", STANDARD.encode(j))).unwrap();
        assert!(n.xray.is_none());
        assert_eq!(n.outbound["tls"]["server_name"], "cdn.example.com");
        assert_eq!(n.outbound["tls"]["alpn"][0], "http/1.1");
    }

    #[test]
    fn ws_host_defaults_to_sni() {
        let n = parse_link("vless://u@1.2.3.4:443?security=tls&type=ws&path=%2F&sni=a.example.com#w").unwrap();
        assert_eq!(n.outbound["transport"]["headers"]["Host"], "a.example.com");
    }

    #[test]
    fn kcp_and_tcp_http_header_go_to_xray() {
        let k = parse_link("vless://u@10.0.0.1:443?type=kcp&headerType=wechat-video&seed=abc#k").unwrap();
        assert_eq!(k.xray.unwrap()["streamSettings"]["kcpSettings"]["seed"], "abc");
        let t = parse_link("vless://u@10.0.0.1:80?type=tcp&headerType=http&host=a.com&path=%2F#t").unwrap();
        assert_eq!(t.xray.unwrap()["streamSettings"]["tcpSettings"]["header"]["type"], "http");
        assert!(parse_link("vless://u@example.com:443?type=grpc&serviceName=s&security=tls#g").unwrap().xray.is_none());
    }

    /// Runs `xray run -test` on the generated outbounds when the binary is available (skipped otherwise).
    #[test]
    fn xray_outbounds_pass_xray_test() {
        let bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries").join(if cfg!(windows) { "xray.exe" } else { "xray" });
        if !bin.exists() {
            eprintln!("skipped: no xray binary");
            return;
        }
        let links = [
            "vless://u-u-i-d@example.com:443?security=tls&type=xhttp&path=%2Fx&host=h.example.com&mode=packet-up&sni=h.example.com&fp=chrome&extra=%7B%22xPaddingBytes%22%3A%22100-1000%22%7D#x",
            "vless://u-u-i-d@example.com:443?security=reality&type=xhttp&sni=a.com&pbk=5Tk7h3m0wZ0yK1xQ2c8dR4vB6nJ9sL1aE3fG7uH0iMo&sid=ab&fp=chrome&path=%2F#r",
            "vless://83baf1d4-f357-4baf-872f-d60f6c2c14fa@10.0.0.1:443?type=kcp&headerType=none&seed=abc#k",
            "vless://83baf1d4-f357-4baf-872f-d60f6c2c14fa@10.0.0.1:80?type=tcp&headerType=http&host=a.com&path=%2F#t",
            "trojan://pw@example.com:443?type=xhttp&security=tls&path=%2F#tr",
        ];
        for l in links {
            let n = parse_link(l).unwrap();
            let mut ob = n.xray.clone().unwrap();
            ob["tag"] = json!("o");
            let cfg = json!({ "outbounds": [ob] });
            let path = std::env::temp_dir().join("vanish-xray-check.json");
            std::fs::write(&path, serde_json::to_vec(&cfg).unwrap()).unwrap();
            let out = std::process::Command::new(&bin).args(["run", "-test", "-c"]).arg(&path).output().unwrap();
            assert!(out.status.success(), "{l}: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        }
    }
}
