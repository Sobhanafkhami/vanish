//! Dynamic sing-box configuration (new-style DNS servers, route actions, WireGuard endpoint: sing-box >= 1.12, verified on 1.14).

use serde_json::{json, Value};

use crate::{
    nodes::Node,
    state::{Mode, RouteMode, Settings},
};

/// WireGuard moved from an outbound to an "endpoint" in recent sing-box releases.
fn wireguard_endpoint(ob: &Value, tag: &str) -> Value {
    let mut peer = json!({
        "address": ob["server"],
        "port": ob["server_port"],
        "public_key": ob["peer_public_key"],
        "allowed_ips": ["0.0.0.0/0", "::/0"],
    });
    if let Some(psk) = ob.get("pre_shared_key") {
        peer["pre_shared_key"] = psk.clone();
    }
    let mut ep = json!({
        "type": "wireguard",
        "tag": tag,
        "address": ob["local_address"],
        "private_key": ob["private_key"],
        "peers": [peer],
    });
    if let Some(mtu) = ob.get("mtu") {
        ep["mtu"] = mtu.clone();
    }
    ep
}

/// Fills in what a share link cannot express but other clients (v2rayNG, v2Box) apply by default, and the
/// dial options that keep a game connection alive. Applied on every build, so saved nodes benefit too.
fn tune_outbound(ob: &mut Value, game: bool) {
    // Without xudp, VLESS UDP (DNS, voice chat, game traffic) is not carried as full-cone and often stalls.
    if ob["type"] == "vless" && ob.get("packet_encoding").is_none() {
        ob["packet_encoding"] = json!("xudp");
    }
    // `path=/ws?ed=2048` is the Xray convention for WebSocket early data: the first payload rides in the
    // handshake, saving a round trip on every new connection. sing-box needs it spelled out.
    if ob["transport"]["type"] == "ws" && ob["transport"].get("max_early_data").is_none() {
        let ed = ob["transport"]["path"]
            .as_str()
            .and_then(|p| p.split_once('?'))
            .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("ed=")))
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0);
        if let Some(ed) = ed {
            ob["transport"]["max_early_data"] = json!(ed);
            ob["transport"]["early_data_header_name"] = json!("Sec-WebSocket-Protocol");
        }
    }
    if game {
        // Short keep-alives hold the NAT mapping open between bursts and notice a dead path quickly.
        ob["tcp_keep_alive"] = json!("30s");
        ob["tcp_keep_alive_interval"] = json!("15s");
        ob["connect_timeout"] = json!("6s");
    }
}

pub fn build(node: &Node, s: &Settings) -> Value {
    let game = s.game_mode;
    let mut outbounds = vec![json!({ "type": "direct", "tag": "direct" })];
    let mut endpoints: Vec<Value> = vec![];
    if node.outbound["type"] == "wireguard" {
        endpoints.push(wireguard_endpoint(&node.outbound, "proxy"));
    } else {
        let mut outbound = node.outbound.clone();
        outbound["tag"] = json!("proxy");
        tune_outbound(&mut outbound, game);
        outbounds.insert(0, outbound);
    }

    let mut inbounds = vec![json!({
        "type": "mixed",
        "tag": "mixed-in",
        "listen": "127.0.0.1",
        "listen_port": s.mixed_port,
    })];
    if s.mode == Mode::Tun {
        inbounds.push(json!({
            "type": "tun",
            "tag": "tun-in",
            "address": ["172.19.0.1/30", "fdfe:dcba:9876::1/126"],
            "auto_route": true,
            "strict_route": true,
            "stack": "mixed",
            // Full-cone NAT: peers can reach the player directly, which online games rely on.
            "endpoint_independent_nat": game,
        }));
    }

    // Routing: private nets always direct, then user-defined bypasses, then the route mode.
    let direct_suffixes: Vec<String> = s
        .direct_domains
        .iter()
        .map(|d| d.trim().trim_start_matches("*.").to_string())
        .filter(|d| !d.is_empty())
        .chain(if s.route_mode == RouteMode::Rule { Some(".ir".to_string()) } else { None })
        .collect();
    // Sniffing is only worth its cost when a rule needs the destination's domain and the inbound cannot
    // supply it (TUN). The mixed inbound already knows the host, and sniffing server-first protocols waits for a timeout.
    let sniff = s.mode == Mode::Tun && !game && !direct_suffixes.is_empty();
    let mut rules = vec![];
    if sniff {
        rules.push(json!({ "action": "sniff" }));
        rules.push(json!({ "protocol": "dns", "action": "hijack-dns" }));
    }
    rules.push(json!({ "port": 53, "action": "hijack-dns" }));
    rules.push(json!({ "ip_is_private": true, "outbound": "direct" }));
    let apps: Vec<&String> = s.direct_apps.iter().filter(|a| !a.trim().is_empty()).collect();
    if !apps.is_empty() {
        rules.push(json!({ "process_name": apps, "outbound": "direct" }));
    }
    let mut dns_rules: Vec<Value> = vec![];
    if !direct_suffixes.is_empty() {
        rules.push(json!({ "domain_suffix": direct_suffixes, "outbound": "direct" }));
        // Resolve bypassed domains locally so they do not leak to / depend on the remote resolver.
        dns_rules.push(json!({ "domain_suffix": direct_suffixes, "action": "route", "server": "local" }));
    }
    // Game mode with a list of games behaves like a route optimizer: only those programs use the tunnel,
    // everything else keeps its own, shorter path.
    let game_apps: Vec<&String> = if game { s.game_apps.iter().filter(|a| !a.trim().is_empty()).collect() } else { vec![] };
    if !game_apps.is_empty() {
        rules.push(json!({ "process_name": game_apps, "outbound": "proxy" }));
    }
    let direct_all = s.route_mode == RouteMode::Direct;
    let route_final_direct = direct_all || !game_apps.is_empty();
    let dns_server = if s.dns_server.trim().is_empty() { "1.1.1.1" } else { s.dns_server.trim() };

    let mut cfg = json!({
        "log": { "level": "info", "timestamp": true },
        "dns": {
            "servers": [
                { "type": "https", "tag": "remote", "server": dns_server, "detour": if direct_all { "direct" } else { "proxy" } },
                { "type": "local", "tag": "local" }
            ],
            "rules": dns_rules,
            "final": "remote",
        },
        "inbounds": inbounds,
        "outbounds": outbounds,
        "endpoints": endpoints,
        "route": {
            "rules": rules,
            "final": if route_final_direct { "direct" } else { "proxy" },
            "auto_detect_interface": true,
            // Resolves the proxy server's own hostname without going through the tunnel.
            "default_domain_resolver": "local",
        },
        "experimental": {
            "clash_api": { "external_controller": format!("127.0.0.1:{}", s.api_port) }
        }
    });
    if game {
        // One lookup per name: AAAA answers rarely help a game and double the wait on a slow resolver.
        cfg["dns"]["strategy"] = json!("ipv4_only");
    }
    cfg
}

/// Headless config for the real-delay probe: every node as its own outbound (`n0`, `n1`, ...) plus the
/// Clash API, no inbounds. Delay checks are made through that API, one per node.
pub fn probe(nodes: &[&Node], api_port: u16) -> Value {
    let mut outbounds = vec![json!({ "type": "direct", "tag": "direct" })];
    let mut endpoints: Vec<Value> = vec![];
    for (i, node) in nodes.iter().enumerate() {
        let tag = format!("n{i}");
        if node.outbound["type"] == "wireguard" {
            endpoints.push(wireguard_endpoint(&node.outbound, &tag));
        } else {
            let mut ob = node.outbound.clone();
            ob["tag"] = json!(tag);
            tune_outbound(&mut ob, false);
            outbounds.push(ob);
        }
    }
    json!({
        "log": { "level": "error", "timestamp": true },
        "dns": { "servers": [{ "type": "local", "tag": "local" }] },
        "outbounds": outbounds,
        "endpoints": endpoints,
        "route": { "final": "direct", "auto_detect_interface": true, "default_domain_resolver": "local" },
        "experimental": { "clash_api": { "external_controller": format!("127.0.0.1:{api_port}") } }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::parse_link;

    /// Runs `sing-box check` on generated configs when a core binary is available
    /// (src-tauri/binaries after `npm run fetch-core`); skipped otherwise.
    #[test]
    fn generated_configs_pass_sing_box_check() {
        let bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries").join(if cfg!(windows) { "sing-box.exe" } else { "sing-box" });
        if !bin.exists() {
            eprintln!("skipped: no sing-box binary");
            return;
        }
        let links = [
            "vless://83baf1d4-f357-4baf-872f-d60f6c2c14fa@example.com:443?security=tls&type=ws&path=%2F&sni=example.com#v",
            "vless://83baf1d4-f357-4baf-872f-d60f6c2c14fa@example.com:443?security=tls&type=ws&path=%2Fws%3Fed%3D2048&sni=example.com#ed",
            "trojan://pw@example.com:443#t",
            "wireguard://cHJpdmF0ZWtleXByaXZhdGVrZXlwcml2YXRla2V5MTI=@example.com:51820?publickey=cHVibGljcHVibGljcHVibGljcHVibGljcHVibGljMTI=&address=10.0.0.2/32#w",
        ];
        for (mode, route_mode) in [(Mode::SystemProxy, RouteMode::Rule), (Mode::Tun, RouteMode::Rule), (Mode::SystemProxy, RouteMode::Global), (Mode::SystemProxy, RouteMode::Direct)] {
            for l in links {
                let node = parse_link(l).unwrap();
                let settings = Settings {
                    mode,
                    route_mode,
                    direct_domains: vec!["example.org".into()],
                    direct_apps: vec!["steam.exe".into()],
                    ..Settings::default()
                };
                let path = std::env::temp_dir().join("vanish-check.json");
                std::fs::write(&path, serde_json::to_vec(&build(&node, &settings)).unwrap()).unwrap();
                let out = std::process::Command::new(&bin).args(["check", "-c"]).arg(&path).output().unwrap();
                assert!(out.status.success(), "{l}: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            }
        }

        // Game mode: TUN, per-app routing, keep-alives.
        for l in links {
            let node = parse_link(l).unwrap();
            for apps in [vec![], vec!["game.exe".to_string()]] {
                let settings = Settings { mode: Mode::Tun, game_mode: true, game_apps: apps, ..Settings::default() };
                let path = std::env::temp_dir().join("vanish-check-game.json");
                std::fs::write(&path, serde_json::to_vec(&build(&node, &settings)).unwrap()).unwrap();
                let out = std::process::Command::new(&bin).args(["check", "-c"]).arg(&path).output().unwrap();
                assert!(out.status.success(), "game {l}: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            }
        }

        // The headless probe config must be valid too (incl. a WireGuard endpoint).
        let nodes: Vec<Node> = links.iter().map(|l| parse_link(l).unwrap()).collect();
        let refs: Vec<&Node> = nodes.iter().collect();
        let path = std::env::temp_dir().join("vanish-probe-check.json");
        std::fs::write(&path, serde_json::to_vec(&probe(&refs, 19999)).unwrap()).unwrap();
        let out = std::process::Command::new(&bin).args(["check", "-c"]).arg(&path).output().unwrap();
        assert!(out.status.success(), "probe: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    }
}
