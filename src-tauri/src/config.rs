//! Dynamic sing-box configuration (targets sing-box >= 1.11 route-action syntax).

use serde_json::{json, Value};

use crate::{
    nodes::Node,
    state::{Mode, Settings},
};

pub fn build(node: &Node, s: &Settings) -> Value {
    let mut outbound = node.outbound.clone();
    outbound["tag"] = json!("proxy");

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

    json!({
        "log": { "level": "info", "timestamp": true },
        "dns": {
            "servers": [
                { "tag": "remote", "address": "https://1.1.1.1/dns-query", "detour": "proxy" },
                { "tag": "local", "address": "local", "detour": "direct" }
            ],
            // Resolve the proxy server's own hostname locally to avoid a routing loop.
            "rules": [{ "outbound": "any", "server": "local" }],
            "final": "remote",
        },
        "inbounds": inbounds,
        "outbounds": [
            outbound,
            { "type": "direct", "tag": "direct" },
            { "type": "block", "tag": "block" }
        ],
        "route": {
            "rules": [
                { "action": "sniff" },
                { "protocol": "dns", "action": "hijack-dns" },
                { "ip_is_private": true, "outbound": "direct" }
            ],
            "final": "proxy",
            "auto_detect_interface": true,
        },
        "experimental": {
            "clash_api": { "external_controller": format!("127.0.0.1:{}", s.api_port) }
        }
    })
}
