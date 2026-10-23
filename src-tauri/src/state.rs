use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tokio::process::Child;

use crate::nodes::Node;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    SystemProxy,
    Tun,
    /// Only expose the local mixed inbound, don't touch the OS.
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    SingBox,
    Xray,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    pub engine: Engine,
    pub mixed_port: u16,
    pub api_port: u16,
    pub core_path: Option<String>,
    pub selected_node: Option<String>,
    pub ping_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::SystemProxy,
            engine: Engine::SingBox,
            mixed_port: 2080,
            api_port: 9090,
            core_path: None,
            selected_node: None,
            ping_url: "https://www.gstatic.com/generate_204".into(),
        }
    }
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub nodes: Mutex<Vec<Node>>,
    pub settings: Mutex<Settings>,
    pub child: tokio::sync::Mutex<Option<Child>>,
    pub traffic_task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub active_node: Mutex<Option<String>>,
}

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

impl AppState {
    pub fn load(data_dir: PathBuf) -> Self {
        let nodes = read_json::<Vec<Node>>(&data_dir.join("nodes.json"));
        let settings = read_json::<Settings>(&data_dir.join("settings.json"));
        Self {
            data_dir,
            nodes: Mutex::new(nodes),
            settings: Mutex::new(settings),
            child: tokio::sync::Mutex::new(None),
            traffic_task: Mutex::new(None),
            active_node: Mutex::new(None),
        }
    }

    pub fn save_nodes(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&*self.nodes.lock().unwrap()).map_err(|e| e.to_string())?;
        fs::write(self.data_dir.join("nodes.json"), json).map_err(|e| e.to_string())
    }

    pub fn save_settings(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&*self.settings.lock().unwrap()).map_err(|e| e.to_string())?;
        fs::write(self.data_dir.join("settings.json"), json).map_err(|e| e.to_string())
    }
}
