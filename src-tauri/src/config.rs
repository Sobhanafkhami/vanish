//! Dynamic sing-box configuration (new-style DNS servers, route actions, WireGuard endpoint: sing-box >= 1.12, verified on 1.14).

use serde_json::{json, Value};

use crate::{
    nodes::Node,
    state::{Mode, RouteMode, Settings},
};

/// WireGuard moved from an outbound to an "endpoint" in recent sing-box releases.
fn wireguard_endpoint(ob: &Value) -> Value {
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
        "tag": "proxy",
        "address": ob["local_address"],
        "private_key": ob["private_key"],
        "peers": [peer],
    });
    if let Some(mtu) = ob.get("mtu") {
        ep["mtu"] = mtu.clone();
    }
    ep
}

pub fn build(node: &Node, s: &Settings) -> Value {
    let mut outbounds = vec![json!({ "type": "direct", "tag": "direct" })];
    let mut endpoints: Vec<Value> = vec![];
    if node.outbound["type"] == "wireguard" {
        endpoints.push(wireguard_endpoint(&node.outbound));
    } else {
        let mut outbound = node.outbound.clone();
        outbound["tag"] = json!("proxy");
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
    let mut rules = vec![
        json!({ "action": "sniff" }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "ip_is_private": true, "outbound": "direct" }),
    ];
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
    let direct_all = s.route_mode == RouteMode::Direct;
    let dns_server = if s.dns_server.trim().is_empty() { "1.1.1.1" } else { s.dns_server.trim() };

    json!({
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
            "final": if direct_all { "direct" } else { "proxy" },
            "auto_detect_interface": true,
            // Resolves the proxy server's own hostname without going through the tunnel.
            "default_domain_resolver": "local",
        },
        "experimental": {
            "clash_api": { "external_controller": format!("127.0.0.1:{}", s.api_port) }
        }
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
    }
}
