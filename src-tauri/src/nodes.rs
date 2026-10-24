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
    })
}

fn tls_from(q: &Query, server: &str, default_tls: bool) -> Option<Value> {
    let sec = q
        .get("security")
        .map(String::as_str)
        .unwrap_or(if default_tls { "tls" } else { "none" });
    if sec == "none" || sec.is_empty() {
        return None;
    }
    let sni = q.get("sni").or(q.get("peer")).or(q.get("host")).cloned().unwrap_or_else(|| server.to_string());
    let mut tls = json!({ "enabled": true, "server_name": sni });
    if matches!(q.get("allowInsecure").or(q.get("insecure")).map(String::as_str), Some("1" | "true")) {
        tls["insecure"] = json!(true);
    }
    if let Some(alpn) = q.get("alpn").filter(|s| !s.is_empty()) {
        tls["alpn"] = json!(alpn.split(',').collect::<Vec<_>>());
    }
    if let Some(fp) = q.get("fp").filter(|s| !s.is_empty()) {
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
    let t = q.get("type").map(String::as_str).unwrap_or("tcp");
    let path = q.get("path").cloned().unwrap_or_else(|| "/".into());
    let host = q.get("host").filter(|s| !s.is_empty()).cloned();
    match t {
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
        "http" | "h2" => {
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
        _ => None,
    }
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
    node_from_outbound(name, ob)
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
    node_from_outbound(name, ob)
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
    node_from_outbound(s("ps"), ob)
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
}
