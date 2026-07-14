#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Manager, Window};
use tauri::ipc::Channel;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreBuilder;
use zip::ZipArchive;
use flate2::read::GzDecoder;

use rand::Rng;
use base64::Engine;
use tokio::sync::Semaphore;
use sysinfo::System;

// -----------------------------------------------
// Constantes
// -----------------------------------------------

const MINECRAFT_VERSION: &str = "1.20.1";
const FORGE_VERSION: &str = "47.4.20";
const INSTANCE_NAME: &str = "BeyondPromisedSparks";
const JAVA_MAJOR: u32 = 25;
const CURSEFORGE_API_KEY: &str = "$2a$10$gMLYaHDAOx.Tcn4rbMN3Gu78UBMZsviJu24bFz3p5MHnQthqm33DK";
const BATCH_SIZE: usize = 20;
const ICON_BATCH_SIZE: usize = 8;

// Modpack: distributed as a "modpack.zip" asset attached to a GitHub
// Release of the same repo. There's no meta.json to maintain: publishing
// a new Release IS the update.
const MODPACK_REPO_OWNER: &str = "SaberY24";
const MODPACK_REPO_NAME: &str = "Beyond-Promised-Sparks";
const MODPACK_ASSET_NAME: &str = "modpack.zip";
// Folders fully controlled by the modpack: files here are added, updated,
// and DELETED so the client ends up identical to the pack. saves/,
// screenshots/, options.txt, servers.dat, logs/, etc. are never touched
// because they aren't in this list.
const MODPACK_MANAGED_DIRS: &[&str] = &["mods", "config", "defaultconfigs", "shaderpacks", "resourcepacks"];

// Forge mod id for Distant Horizons (read from its META-INF/mods.toml). Used
// by the "Modpack Profile" feature (Default / No DH) to find the mod
// regardless of which version/jar filename is currently installed
// (e.g. DistantHorizons-3.0.3-b-1.20.1-fabric-forge.jar vs
// DistantHorizons-3.1.2-b-1.20.1-fabric-forge.jar). The filename prefix
// check is a fallback in case the mod id can't be parsed for some reason.
const DISTANT_HORIZONS_MOD_ID: &str = "distanthorizons";

fn is_distant_horizons(file_name: &str, mod_id: &str) -> bool {
    mod_id.eq_ignore_ascii_case(DISTANT_HORIZONS_MOD_ID)
        || file_name.to_lowercase().starts_with("distanthorizons-")
}

// -----------------------------------------------
// Data structures
// -----------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub account_type: String,
    pub refresh_token: Option<String>,
    pub skin_url: Option<String>,
    pub skin_texture_key: Option<String>,
}

#[derive(Debug, Clone)]
struct MinecraftLoginFlow {
    verifier: String,
    #[allow(dead_code)]
    challenge: String,
    auth_request_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct XboxLiveAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Serialize, Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct XstsResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinecraftAuthResponse {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
    skins: Option<Vec<MinecraftSkin>>,
    capes: Option<Vec<MinecraftCape>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinecraftSkin {
    id: String,
    state: String,
    url: String,
    #[serde(rename = "texture_key")]
    texture_key: Option<String>,
    variant: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MinecraftCape {
    id: String,
    state: String,
    url: String,
    alias: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CustomPreset {
    hex: String,
    #[serde(default)]
    name: String,
}

// Antes `custom_presets` se guardaba como un array plano de hex strings (sin
// nombre). Para no romper instalaciones existentes al agregar nombres a los
// presets, aceptamos ambos formatos y normalizamos todo a CustomPreset.
fn deserialize_custom_presets<'de, D>(deserializer: D) -> Result<Vec<CustomPreset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawPreset {
        Legacy(String),
        Named { hex: String, #[serde(default)] name: String },
    }

    let raw: Vec<RawPreset> = Vec::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .enumerate()
        .map(|(i, p)| match p {
            RawPreset::Legacy(hex) => CustomPreset { hex, name: format!("Custom {}", i + 1) },
            RawPreset::Named { hex, name } => {
                let name = if name.trim().is_empty() { format!("Custom {}", i + 1) } else { name };
                CustomPreset { hex, name }
            }
        })
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    ram: i32,
    java_path: String,
    game_dir: String,
    resolution: String,
    fullscreen: bool,
    custom_titlebar: bool,
    #[serde(default)]
    theme: String,
    #[serde(default)]
    accent_color: String,
    #[serde(default, deserialize_with = "deserialize_custom_presets")]
    custom_presets: Vec<CustomPreset>,
    #[serde(default)]
    java_args: String,
    #[serde(default)]
    curseforge_api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuthToken {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    pub stage: String,
    pub current: u64,
    pub total: u64,
    pub percent: f32,
    pub detail: String,
    pub speed: f64,
    pub eta: Option<Duration>,
}

// -----------------------------------------------
// Global state of the running game process
// -----------------------------------------------
// We only store the PID (not the full Child) so we don't block the thread that
// waits for the game to finish (launch_game_impl still holds the only
// reference to the Child and calls wait_with_output() normally). The frontend's
// "Stop" button uses the PID to kill the process from outside.
#[derive(Default)]
struct GameProcessState {
    pid: std::sync::Mutex<Option<u32>>,
    stop_requested: std::sync::Mutex<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceJson {
    pub minecraft_version: String,
    pub forge_version: String,
    pub java_version: u32,
    pub installed: bool,
    pub min_ram: u64,
    pub max_ram: u64,
    pub play_count: u64,
    pub last_played: Option<String>,
    // Accumulated playtime in seconds. `#[serde(default)]` so instance.json
    // files written before this field existed still deserialize fine.
    #[serde(default)]
    pub total_time_played_secs: u64,
}

impl Default for InstanceJson {
    fn default() -> Self {
        Self {
            minecraft_version: MINECRAFT_VERSION.to_string(),
            forge_version: FORGE_VERSION.to_string(),
            java_version: JAVA_MAJOR,
            installed: false,
            min_ram: 512,
            max_ram: 4096,
            play_count: 0,
            last_played: None,
            total_time_played_secs: 0,
        }
    }
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub file_name: String,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub fingerprint: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModCacheEntry {
    size: u64,
    modified: u64,
    info: ModInfo,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // logo_file se parsea desde el TOML pero aún no se usa (posible ícono futuro)
struct ModsToml {
    #[serde(rename = "logoFile", default)]
    logo_file: Option<String>,
    #[serde(default)]
    mods: Vec<ModTomlEntry>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // logo_file se parsea desde el TOML pero aún no se usa (posible ícono futuro)
struct ModTomlEntry {
    #[serde(rename = "modId", default)]
    mod_id: String,
    #[serde(default)]
    version: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "logoFile", default)]
    logo_file: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionManifest {
    versions: Vec<ManifestVersion>,
}

#[derive(Debug, Deserialize)]
struct ManifestVersion {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[allow(dead_code)] // size viene en el manifest de Mojang; se parsea pero no se usa en la descarga (se valida por sha1)
struct Artifact {
    #[serde(default)]
    path: Option<String>,
    sha1: String,
    size: u64,
    url: String,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)] // size/total_size vienen en el manifest de assets; se parsean pero no se usan (se valida por sha1)
struct AssetIndex {
    id: String,
    sha1: String,
    size: u64,
    #[serde(default, rename = "totalSize")]
    total_size: u64,
    url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Library {
    name: String,
    downloads: LibraryDownloads,
    #[serde(default)]
    natives: Option<HashMap<String, String>>,
    #[serde(default)]
    rules: Option<Vec<Rule>>,
}

#[derive(Debug, Deserialize, Clone)]
struct LibraryDownloads {
    artifact: Option<Artifact>,
    classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Debug, Deserialize, Clone)]
struct Rule {
    action: String,
    os: Option<OsRule>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)] // version se parsea desde las reglas del manifest pero no se evalúa (solo se usa `name`)
struct OsRule {
    name: String,
    #[serde(default)]
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstallProfile {
    #[serde(rename = "version")]
    _version: String,
    #[serde(default)]
    libraries: Vec<Library>,
    #[serde(default)]
    processors: Vec<Processor>,
    #[serde(default)]
    data: HashMap<String, DataEntry>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // outputs se parsea del install profile de Forge pero no se valida tras ejecutar el processor
struct Processor {
    jar: String,
    #[serde(default)]
    classpath: Vec<String>,
    #[serde(default)]
    args: Value,
    #[serde(default)]
    outputs: Option<HashMap<String, String>>,
    #[serde(default)]
    sides: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // server se parsea del install profile pero el launcher solo instala el lado cliente
struct DataEntry {
    client: Option<String>,
    server: Option<String>,
}

// -----------------------------------------------
// Helper functions
// -----------------------------------------------

fn sanitize_component(comp: &str) -> String {
    comp
        .replace('\\', "_")
        .replace('/', "_")
        .replace(':', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
        .trim_end_matches('.')
        .trim_end_matches(' ')
        .to_string()
}

fn is_file_valid(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() == 0 {
            return false;
        }
    }
    true
}

fn is_json_valid(path: &Path) -> bool {
    if !is_file_valid(path) {
        return false;
    }
    if let Ok(file) = File::open(path) {
        if serde_json::from_reader::<_, serde_json::Value>(file).is_ok() {
            return true;
        }
    }
    false
}

fn assets_seem_complete(shared_dir: &Path, version_json_path: &Path) -> bool {
    let Ok(data) = fs::read_to_string(version_json_path) else { return false; };
    let Ok(json) = serde_json::from_str::<Value>(&data) else { return false; };
    let Some(asset_index_id) = json["assetIndex"]["id"].as_str() else { return false; };

    let assets_dir = shared_dir.join("assets");
    let index_path = assets_dir.join("indexes").join(format!("{}.json", asset_index_id));
    let Ok(index_data) = fs::read_to_string(&index_path) else { return false; };
    let Ok(index_json) = serde_json::from_str::<Value>(&index_data) else { return false; };
    let Some(objects) = index_json["objects"].as_object() else { return false; };

    for obj in objects.values() {
        let Some(hash) = obj["hash"].as_str() else { continue; };
        if hash.len() < 2 { continue; }
        let dest = assets_dir.join("objects").join(&hash[0..2]).join(hash);
        if !is_file_valid(&dest) {
            return false;
        }
    }
    true
}

fn generate_oauth_challenge() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn generate_pkce() -> (String, String) {
    let verifier = generate_oauth_challenge();
    let mut hasher = sha2::Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(hasher.finalize());
    (verifier, challenge)
}

/// Decodifica correctamente un valor de query string: primero resuelve las
/// secuencias %XX (percent-encoding) y luego trata '+' como espacio.
/// El código de autorización de Microsoft puede contener caracteres como
/// '+', '/' o '=' que llegan percent-encoded (%2B, %2F, %3D) en la URL;
/// sin este decode se enviaba el código "crudo" (con %XX literales) al
/// endpoint de token, lo que produce "invalid_grant: The provided value
/// for the 'code' parameter is not valid".
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Debug, Clone, Serialize)]
struct DirEntry {
    name: String,
    path: String,
}

#[tauri::command]
async fn read_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut entries = Vec::new();
    let dir = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    for entry in dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
        });
    }
    Ok(entries)
}

#[tauri::command]
fn file_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

#[tauri::command]
async fn detect_java_25() -> Result<Option<String>, String> {
    let platform = std::env::consts::OS;

    if platform == "windows" {
        let search_roots = [
            r"C:\Program Files\Java",
            r"C:\Program Files (x86)\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\Amazon Corretto",
            r"C:\Program Files\Zulu",
            r"C:\Program Files\BellSoft",
            r"C:\Program Files\AdoptOpenJDK",
            r"C:\Program Files\Semeru",
        ];

        for root in &search_roots {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains("25") || name.contains("jdk") || name.contains("java") {
                        let path = entry.path();
                        let candidate = path.join("bin").join("javaw.exe");
                        if candidate.exists() {
                            if let Ok(output) = std::process::Command::new(&candidate)
                                .args(["-version"])
                                .output()
                            {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                if stderr.contains("25") || stderr.contains("21") {
                                    return Ok(Some(candidate.to_string_lossy().to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let candidate = std::path::Path::new(&java_home).join("bin").join("javaw.exe");
            if candidate.exists() {
                return Ok(Some(candidate.to_string_lossy().to_string()));
            }
        }

        if let Ok(path_var) = std::env::var("PATH") {
            for path_dir in path_var.split(';') {
                let candidate = std::path::Path::new(path_dir).join("javaw.exe");
                if candidate.exists() {
                    return Ok(Some(candidate.to_string_lossy().to_string()));
                }
            }
        }
    } else if platform == "macos" {
        let search_paths = [
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
            "/usr/local/opt/openjdk@25",
            "/opt/homebrew/opt/openjdk@25",
            "/usr/local/opt/openjdk@21",
            "/opt/homebrew/opt/openjdk@21",
        ];

        for base in &search_paths {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains("25") || name.contains("jdk") {
                        let path = entry.path();
                        let candidate = path.join("Contents").join("Home").join("bin").join("java");
                        if candidate.exists() {
                            return Ok(Some(candidate.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }

        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let candidate = std::path::Path::new(&java_home).join("bin").join("java");
            if candidate.exists() {
                return Ok(Some(candidate.to_string_lossy().to_string()));
            }
        }
    } else {
        // Linux
        let search_paths = [
            "/usr/lib/jvm",
            "/usr/java",
            "/opt/java",
            "/usr/local/java",
            "/snap",
        ];

        for base in &search_paths {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains("25") || name.contains("jdk") || name.contains("java") {
                        let path = entry.path();
                        let candidate = path.join("bin").join("java");
                        if candidate.exists() {
                            return Ok(Some(candidate.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }

        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            let candidate = std::path::Path::new(&java_home).join("bin").join("java");
            if candidate.exists() {
                return Ok(Some(candidate.to_string_lossy().to_string()));
            }
        }
    }

    Ok(None)
}

// -----------------------------------------------
// System info
// -----------------------------------------------

#[tauri::command]
fn get_system_ram_mb() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_memory();
    // sysinfo (>=0.30) reports total_memory() in bytes.
    sys.total_memory() / 1024 / 1024
}

// -----------------------------------------------
// Window decorations
// -----------------------------------------------

#[tauri::command]
async fn set_window_decorations(app: AppHandle, decorations: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_decorations(decorations)
            .map_err(|e| format!("Failed to set decorations: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn get_window_decorations(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        window.is_decorated()
            .map_err(|e| format!("Failed to get decorations: {}", e))
    } else {
        Ok(false)
    }
}

// -----------------------------------------------
// Login
// -----------------------------------------------

#[tauri::command]
async fn login_offline(username: String) -> Result<Account, String> {
    if username.len() < 3 || username.len() > 16 {
        return Err("Username must be 3-16 characters".to_string());
    }
    let hash = username.as_bytes().iter().fold(0u64, |a, b| a.wrapping_add(*b as u64));
    let hash_hex = format!("{:08x}", hash);
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hash_hex[..8],
        "0000", "0000", "0000", "000000000000"
    );
    Ok(Account {
        id: format!("offline-{}", username),
        username,
        uuid,
        access_token: "offline".to_string(),
        account_type: "offline".to_string(),
        refresh_token: None,
        skin_url: None,
        skin_texture_key: None,
    })
}

async fn xbox_minecraft_flow(ms_access_token: String) -> Result<Account, String> {
    let client = reqwest::Client::new();

    let xbl_body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", ms_access_token)
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let xbl_res = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&xbl_body)
        .send()
        .await
        .map_err(|e| format!("XBL request failed: {}", e))?;

    let xbl_status = xbl_res.status();
    let xbl_text = xbl_res.text().await.map_err(|e| e.to_string())?;
    if !xbl_status.is_success() {
        return Err(format!("XBL auth failed ({}): {}", xbl_status, xbl_text));
    }

    let xbl_json: XboxLiveAuthResponse = serde_json::from_str(&xbl_text)
        .map_err(|e| format!("XBL JSON decode error: {} | body: {}", e, xbl_text))?;

    let xbl_token = xbl_json.token;
    let uhs = xbl_json
        .display_claims
        .xui
        .get(0)
        .map(|x| x.uhs.clone())
        .unwrap_or_default();

    if uhs.is_empty() {
        return Err("XBL: missing UHS claim".to_string());
    }

    let xsts_body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbl_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let xsts_res = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&xsts_body)
        .send()
        .await
        .map_err(|e| format!("XSTS request failed: {}", e))?;

    let xsts_status = xsts_res.status();
    let xsts_text = xsts_res.text().await.map_err(|e| e.to_string())?;
    if !xsts_status.is_success() {
        return Err(format!("XSTS auth failed ({}): {}", xsts_status, xsts_text));
    }

    let xsts_json: XstsResponse = serde_json::from_str(&xsts_text)
        .map_err(|e| format!("XSTS JSON decode error: {} | body: {}", e, xsts_text))?;

    let xsts_token = xsts_json.token;

    let mc_body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
    });

    let mc_auth_res = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&mc_body)
        .send()
        .await
        .map_err(|e| format!("MC auth request failed: {}", e))?;

    let mc_auth_status = mc_auth_res.status();
    let mc_auth_text = mc_auth_res.text().await.map_err(|e| e.to_string())?;
    if !mc_auth_status.is_success() {
        return Err(format!("MC auth failed ({}): {}", mc_auth_status, mc_auth_text));
    }

    let mc_auth: MinecraftAuthResponse = serde_json::from_str(&mc_auth_text)
        .map_err(|e| format!("MC auth JSON decode error: {} | body: {}", e, mc_auth_text))?;

    let mc_access_token = mc_auth.access_token;

    let profile_res = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc_access_token)
        .send()
        .await
        .map_err(|e| format!("Profile request failed: {}", e))?;

    let profile_status = profile_res.status();
    let profile_text = profile_res.text().await.map_err(|e| e.to_string())?;
    if !profile_status.is_success() {
        return Err(format!("Profile fetch failed ({}): {}", profile_status, profile_text));
    }

    let profile: MinecraftProfile = serde_json::from_str(&profile_text)
        .map_err(|e| format!("Profile JSON decode error: {} | body: {}", e, profile_text))?;

    let active_skin = profile.skins.as_ref().and_then(|skins| {
        skins.iter().find(|s| s.state == "ACTIVE")
    });

    let skin_url = active_skin.map(|s| s.url.clone());
    let skin_texture_key = active_skin.and_then(|s| s.texture_key.clone());

    Ok(Account {
        id: format!("ms-{}", profile.id),
        username: profile.name,
        uuid: profile.id,
        access_token: mc_access_token,
        account_type: "microsoft".to_string(),
        refresh_token: None,
        skin_url,
        skin_texture_key,
    })
}

async fn oauth_token(code: &str, verifier: &str) -> Result<OAuthToken, String> {
    let client = reqwest::Client::new();

    let res = client
        .post("https://login.live.com/oauth20_token.srf")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", "00000000402b5328"),
            ("code", code),
            ("code_verifier", verifier),
            ("grant_type", "authorization_code"),
            ("redirect_uri", "https://login.live.com/oauth20_desktop.srf"),
        ])
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;

    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Token exchange failed ({}): {}", status, text));
    }

    let body: OAuthToken = serde_json::from_str(&text)
        .map_err(|e| format!("Token JSON decode error: {} | body: {}", e, text))?;

    Ok(body)
}

// TODO: nunca se llama todavía. La cuenta guarda `refresh_token` pero no
// hay ningún sitio que detecte el access_token expirado y llame a esta
// función para renovarlo — hoy, cuando expira, el usuario tiene que volver
// a loguearse manualmente. Conectarlo bien requiere guardar cuándo expira
// el token (Account no tiene ese campo) y revisarlo antes de lanzar el
// juego o al iniciar la app. Lo dejo señalado en vez de borrarlo o
// silenciarlo en falso: no lo conecté solo porque cambia el flujo de auth
// y merece probarse aparte, no colarlo en una limpieza de warnings.
#[allow(dead_code)]
async fn refresh_microsoft_token(refresh_token: &str) -> Result<OAuthToken, String> {
    let client = reqwest::Client::new();

    let res = client
        .post("https://login.live.com/oauth20_token.srf")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", "00000000402b5328"),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await
        .map_err(|e| format!("Refresh token request failed: {}", e))?;

    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Token refresh failed ({}): {}", status, text));
    }

    let body: OAuthToken = serde_json::from_str(&text)
        .map_err(|e| format!("Refresh token JSON decode error: {} | body: {}", e, text))?;

    Ok(body)
}

async fn login_finish(code: &str, flow: &MinecraftLoginFlow) -> Result<(Account, OAuthToken), String> {
    let oauth_token = oauth_token(code, &flow.verifier).await?;
    let mut account = xbox_minecraft_flow(oauth_token.access_token.clone()).await?;
    account.refresh_token = oauth_token.refresh_token.clone();
    Ok((account, oauth_token))
}

async fn login_begin() -> Result<MinecraftLoginFlow, String> {
    let (verifier, challenge) = generate_pkce();

    let scope = "XboxLive.signin offline_access";

    let auth_request_uri = format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&prompt=select_account",
        "00000000402b5328",
        "https://login.live.com/oauth20_desktop.srf",
        scope.replace(' ', "%20"),
        challenge
    );

    Ok(MinecraftLoginFlow {
        verifier,
        challenge,
        auth_request_uri,
    })
}

#[tauri::command]
async fn login_microsoft(app: AppHandle) -> Result<Account, String> {
    let flow = login_begin().await?;

    let auth_window = tauri::WebviewWindowBuilder::new(
        &app,
        "ms-auth",
        tauri::WebviewUrl::External(flow.auth_request_uri.parse().unwrap())
    )
    .title("Microsoft Sign In")
    .inner_size(480.0, 640.0)
    .min_inner_size(480.0, 640.0)
    .max_inner_size(480.0, 640.0)
    .center()
    .resizable(false)
    .minimizable(false)
    .maximizable(false)
    .closable(true)
    .decorations(true)
    .visible(true)
    .build()
    .map_err(|e| format!("Failed to create auth window: {}", e))?;

    let start = std::time::Instant::now();
    let mut auth_code: Option<String> = None;
    let mut already_captured = false;

    while start.elapsed() < Duration::from_secs(600) {
        tokio::time::sleep(Duration::from_millis(200)).await;

        if auth_window.url().is_err() {
            return Err("Authentication cancelled".to_string());
        }

        let current_url = match auth_window.url() {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };

        if current_url.starts_with("https://login.live.com/oauth20_desktop.srf") {
            if already_captured {
                break;
            }
            already_captured = true;

            if let Some(query) = current_url.split('?').nth(1) {
                let mut code = None;
                let mut error = None;
                let mut error_desc = String::new();

                for pair in query.split('&') {
                    let mut kv = pair.splitn(2, '=');
                    if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                        let v_decoded = percent_decode(v);
                        match k {
                            "code" => code = Some(v_decoded),
                            "error" => error = Some(v_decoded),
                            "error_description" => error_desc = v_decoded,
                            _ => {}
                        }
                    }
                }

                if let Some(e) = error {
                    auth_window.close().ok();
                    return Err(format!("Microsoft auth error: {} - {}", e, error_desc));
                }

                if let Some(c) = code {
                    auth_code = Some(c);
                    auth_window.close().ok();
                    break;
                }
            }
        }
    }

    auth_window.close().ok();

    let code = auth_code.ok_or("Authentication timed out or was cancelled")?;

    let (account, _oauth_token) = login_finish(&code, &flow).await?;

    let session_data = serde_json::json!({
        "accounts": [account.clone()],
        "activeIndex": 0,
    });

    let store = StoreBuilder::new(&app, PathBuf::from("session.json"))
        .build()
        .map_err(|e| e.to_string())?;
    store.set("session".to_string(), session_data);
    store.save().map_err(|e| e.to_string())?;

    Ok(account)
}

// -----------------------------------------------
// Session & settings
// -----------------------------------------------

#[tauri::command]
async fn save_session(app: AppHandle, accounts: Vec<Account>, active_index: usize) -> Result<(), String> {
    let store = StoreBuilder::new(&app, PathBuf::from("session.json"))
        .build()
        .map_err(|e| e.to_string())?;

    let session_data = serde_json::json!({
        "accounts": accounts,
        "activeIndex": active_index,
    });

    store.set("session".to_string(), session_data);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_session(app: AppHandle) -> Result<Option<serde_json::Value>, String> {
    let store = StoreBuilder::new(&app, PathBuf::from("session.json"))
        .build()
        .map_err(|e| e.to_string())?;

    let session = store.get("session").map(|v| v.clone());
    Ok(session)
}

#[tauri::command]
async fn clear_session(app: AppHandle) -> Result<(), String> {
    let store = StoreBuilder::new(&app, PathBuf::from("session.json"))
        .build()
        .map_err(|e| e.to_string())?;
    store.delete("session");
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn scan_java() -> Result<Option<String>, String> {
    let candidates = if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Java\jdk-25\bin\java.exe",
            r"C:\Program Files\Java\jdk-21\bin\java.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-25\bin\java.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-21\bin\java.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Library/Java/JavaVirtualMachines/temurin-25.jdk/Contents/Home/bin/java",
            "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home/bin/java",
            "/usr/bin/java",
        ]
    } else {
        vec!["/usr/lib/jvm/java-25-openjdk/bin/java", "/usr/bin/java"]
    };

    for path in candidates {
        if std::path::Path::new(path).exists() {
            return Ok(Some(path.to_string()));
        }
    }
    Ok(None)
}

#[tauri::command]
async fn browse_java(app: AppHandle) -> Result<Option<String>, String> {
    let path = app.dialog().file().blocking_pick_file();
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command]
async fn browse_game_dir(app: AppHandle) -> Result<Option<String>, String> {
    let path = app.dialog().file().blocking_pick_folder();
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command]
async fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let store = StoreBuilder::new(&app, PathBuf::from("settings.json"))
        .build()
        .map_err(|e| e.to_string())?;
    store.set("settings".to_string(), serde_json::json!(settings));
    store.save().map_err(|e| e.to_string())?;
    
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_decorations(!settings.custom_titlebar);
    }
    
    Ok(())
}

#[tauri::command]
async fn load_settings(app: AppHandle) -> Result<Settings, String> {
    let store = StoreBuilder::new(&app, PathBuf::from("settings.json"))
        .build()
        .map_err(|e| e.to_string())?;
    
    let settings: Option<Settings> = store
        .get("settings")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    
    Ok(settings.unwrap_or(Settings {
        ram: 4096,
        java_path: String::new(),
        game_dir: String::new(),
        resolution: "854x480".to_string(),
        fullscreen: false,
        custom_titlebar: true,
        theme: "system".to_string(),
        accent_color: "#c084fc".to_string(),
        custom_presets: Vec::new(),
        java_args: String::new(),
        curseforge_api_key: String::new(),
    }))
}

#[tauri::command]
async fn launch_game_legacy(
    version: String,
    ram: i32,
    java_path: Option<String>,
    game_dir: Option<String>,
) -> Result<(), String> {
    println!("Launching {} with {}MB RAM", version, ram);
    println!("Java: {:?}, GameDir: {:?}", java_path, game_dir);
    Ok(())
}

// -----------------------------------------------
// HELPER FUNCTIONS FOR PROCESSING FORGE
// -----------------------------------------------

fn library_applies(lib: &Library) -> bool {
    let rules = match &lib.rules {
        Some(r) => r,
        None => return true,
    };
    let os_name = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let mut allow = false;
    for rule in rules {
        let os_match = match &rule.os {
            Some(os) => os.name == os_name,
            None => true,
        };
        if os_match {
            if rule.action == "allow" {
                allow = true;
            } else if rule.action == "disallow" {
                return false;
            }
        }
    }
    allow
}

fn deduplicate_libraries(libraries: Vec<Library>) -> Vec<Library> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for lib in libraries {
        if seen.insert(lib.name.clone()) {
            result.push(lib);
        }
    }
    result
}

fn compute_sha1(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha1::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

fn get_app_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("Sparkle")
        } else {
            app_handle
                .path()
                .app_data_dir()
                .map_err(|e| anyhow!("failed to get app data dir: {}", e))?
        }
    } else {
        app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow!("failed to get app data dir: {}", e))?
    };

    if !base.exists() {
        fs::create_dir_all(&base)?;
    }
    Ok(base)
}

fn get_shared_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("Shared")
}

fn get_instances_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("Instances")
}

fn get_instance_dir(instances_dir: &Path) -> PathBuf {
    instances_dir.join(INSTANCE_NAME)
}

// -----------------------------------------------
// Playtime tracking
// -----------------------------------------------
// Adds `seconds` to the instance's accumulated playtime. Re-reads the JSON
// from disk right before writing (instead of reusing an in-memory copy) so a
// checkpoint written mid-session doesn't clobber other fields that might
// have changed meanwhile (play_count, last_played, settings, etc).
fn add_instance_playtime(instance_path: &Path, seconds: u64) -> Result<()> {
    if seconds == 0 || !instance_path.exists() {
        return Ok(());
    }
    let data = fs::read_to_string(instance_path)
        .with_context(|| format!("Could not read '{}'", instance_path.display()))?;
    let mut instance: InstanceJson = serde_json::from_str(&data)
        .with_context(|| format!("Could not parse '{}'", instance_path.display()))?;
    instance.total_time_played_secs = instance.total_time_played_secs.saturating_add(seconds);
    fs::write(instance_path, serde_json::to_string_pretty(&instance)?)
        .with_context(|| format!("Could not write '{}'", instance_path.display()))?;
    Ok(())
}

fn get_instance_json_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("instance.json")
}

async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    channel: Option<&Channel<InstallProgress>>,
    stage: &str,
    detail: &str,
) -> Result<()> {
    if is_file_valid(dest) {
        if let Some(sha1) = expected_sha1 {
            let actual = compute_sha1(dest)?;
            if actual == sha1 {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }

    if dest.exists() {
        fs::remove_file(dest)?;
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for '{}'", dest.display()))?;
    }

    let start = Instant::now();
    let mut response = client.get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_update = Instant::now();

    // RAM FIX: stream straight to file + hash on the fly (no Vec in memory)
    let file = std::fs::File::create(dest)
        .with_context(|| format!("Failed to create file '{}'", dest.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    let mut hasher = Sha1::new();

    while let Some(chunk) = response.chunk().await? {
        writer.write_all(&chunk)?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        if last_update.elapsed() >= Duration::from_millis(200) {
            let elapsed = start.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { downloaded as f64 / elapsed } else { 0.0 };
            let percent = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };
            let eta = if speed > 0.0 && total_size > downloaded {
                Some(Duration::from_secs_f64((total_size - downloaded) as f64 / speed))
            } else {
                None
            };
            if let Some(chan) = channel {
                let _ = chan.send(InstallProgress {
                    stage: stage.to_string(),
                    current: downloaded,
                    total: total_size,
                    percent,
                    detail: detail.to_string(),
                    speed,
                    eta,
                });
            }
            last_update = Instant::now();
        }
    }

    writer.flush()?;
    drop(writer); // Close the file before verifying

    if downloaded == 0 {
        let _ = fs::remove_file(dest);
        return Err(anyhow!("Downloaded file is empty: {}", dest.display()));
    }

    if let Some(sha1) = expected_sha1 {
        let actual = hex::encode(hasher.finalize());
        if actual != sha1 {
            let _ = fs::remove_file(dest);
            return Err(anyhow!("SHA1 mismatch for {}", dest.display()));
        }
    }

    Ok(())
}

async fn send_progress(channel: &Channel<InstallProgress>, stage: &str, current: u64, total: u64, percent: f32, detail: &str) {
    let _ = channel.send(InstallProgress {
        stage: stage.to_string(),
        current,
        total,
        percent,
        detail: detail.to_string(),
        speed: 0.0,
        eta: None,
    });
}

fn create_directories(shared_dir: &Path, instances_dir: &Path, instance_dir: &Path) -> Result<()> {
    for dir in [
        shared_dir,
        &shared_dir.join("runtime"),
        &shared_dir.join("assets"),
        &shared_dir.join("libraries"),
        &shared_dir.join("versions"),
        &shared_dir.join("cache"),
        &shared_dir.join("logs"),
        &shared_dir.join("log_configs"),
        instances_dir,
        instance_dir,
        &instance_dir.join("mods"),
        &instance_dir.join("config"),
        &instance_dir.join("resourcepacks"),
        &instance_dir.join("shaderpacks"),
        &instance_dir.join("logs"),
        &instance_dir.join("saves"),
        &instance_dir.join("screenshots"),
        &instance_dir.join("crash-reports"),
        &instance_dir.join("natives"),
    ] {
        eprintln!("Creating directory: {}", dir.display());
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }
    Ok(())
}

async fn download_version_manifest(client: &Client) -> Result<VersionManifest> {
    let url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
    let response = client.get(url).send().await?;
    let text = response.text().await?;
    if text.is_empty() {
        return Err(anyhow!("Empty response from version manifest URL"));
    }
    let manifest: VersionManifest = serde_json::from_str(&text)?;
    Ok(manifest)
}

fn extract_forge_metadata(installer_path: &Path, target_dir: &Path) -> Result<()> {
    eprintln!("Opening forge installer: {}", installer_path.display());
    let file = File::open(installer_path)
        .with_context(|| format!("Failed to open forge installer at '{}'", installer_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name == "install_profile.json" || name == "version.json" {
            let dest = target_dir.join(&name);
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            fs::write(&dest, &data)?;
            eprintln!("Extracted {} to {}", name, dest.display());
        }
    }
    Ok(())
}

async fn download_libraries(
    client: &Client,
    shared_dir: &Path,
    libraries: &[Library],
    channel: &Channel<InstallProgress>,
) -> Result<()> {
    let total = libraries.len() as u64;
    let mut downloaded = 0;
    for lib in libraries {
        downloaded += 1;
        let progress = downloaded as f32 / total as f32 * 100.0;
        let name = lib.name.split(':').last().unwrap_or("unknown");
        let _ = channel.send(InstallProgress {
            stage: "Downloading libraries".to_string(),
            current: downloaded,
            total,
            percent: progress,
            detail: format!("{} ({}/{})", name, downloaded, total),
            speed: 0.0,
            eta: None,
        });

        if let Some(artifact) = &lib.downloads.artifact {
            if let Some(path) = &artifact.path {
                let dest = shared_dir.join("libraries").join(path);
                eprintln!("Downloading library: {} -> {}", path, dest.display());
                let expected_sha1 = Some(artifact.sha1.as_str());
                download_file(client, &artifact.url, &dest, expected_sha1, Some(channel), "Downloading libraries", name).await?;
            }
        }

        if let Some(classifiers) = &lib.downloads.classifiers {
            for (_, artifact) in classifiers {
                if let Some(path) = &artifact.path {
                    let dest = shared_dir.join("libraries").join(path);
                    eprintln!("Downloading classifier: {} -> {}", path, dest.display());
                    let expected_sha1 = Some(artifact.sha1.as_str());
                    download_file(client, &artifact.url, &dest, expected_sha1, Some(channel), "Downloading libraries", name).await?;
                }
            }
        }
    }
    Ok(())
}

async fn download_assets(
    client: &Client,
    asset_index: &AssetIndex,
    assets_dir: &Path,
    channel: &Channel<InstallProgress>,
) -> Result<()> {
    let indexes_dir = assets_dir.join("indexes");
    fs::create_dir_all(&indexes_dir)?;
    let objects_dir = assets_dir.join("objects");
    fs::create_dir_all(&objects_dir)?;

    let index_path = indexes_dir.join(format!("{}.json", asset_index.id));
    if !is_file_valid(&index_path) || compute_sha1(&index_path)? != asset_index.sha1 {
        download_file(
            client,
            &asset_index.url,
            &index_path,
            Some(&asset_index.sha1),
            Some(channel),
            "Downloading asset index",
            &format!("{}.json", asset_index.id),
        ).await?;
    }

    let index_content = fs::read_to_string(&index_path)?;
    let index_json: serde_json::Value = serde_json::from_str(&index_content)?;
    let objects = index_json["objects"].as_object().ok_or_else(|| anyhow!("No objects in asset index"))?;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(20));
    let mut tasks = Vec::new();

    for (path, obj) in objects {
        let hash = obj["hash"].as_str().unwrap_or("");
        if hash.is_empty() { continue; }
        
        let dest = objects_dir.join(&hash[0..2]).join(hash);
        
        let needs_download = if dest.exists() {
            match compute_sha1(&dest) {
                Ok(actual) => actual != hash,
                Err(_) => true,
            }
        } else {
            true
        };

        if !needs_download {
            continue;
        }

        let url = format!("https://resources.download.minecraft.net/{}/{}", &hash[0..2], hash);
        let client = client.clone();
        let channel = channel.clone();
        let sem = semaphore.clone();
        let path_clone = path.to_string();
        let hash_clone = hash.to_string();
        
        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let mut last_err = None;
            for attempt in 1..=5 {
                match download_file(&client, &url, &dest, Some(&hash_clone), Some(&channel), "Downloading assets", &path_clone).await {
                    Ok(()) => {
                        eprintln!("Asset downloaded: {} (attempt {}/5)", path_clone, attempt);
                        return Ok(path_clone);
                    }
                    Err(e) => {
                        eprintln!("Asset download attempt {}/5 failed for {}: {}", attempt, path_clone, e);
                        last_err = Some(e);
                        if attempt < 5 {
                            tokio::time::sleep(Duration::from_millis(1000 * attempt as u64)).await;
                        }
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| anyhow!("Unknown error downloading {}", path_clone)))
        });
        tasks.push(task);
    }

    let mut failed: Vec<String> = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                eprintln!("Asset download error (after retries): {}", e);
                failed.push(e.to_string());
            }
            Err(e) => {
                eprintln!("Asset download task panicked/cancelled: {}", e);
                failed.push(e.to_string());
            }
        }
    }

    if !failed.is_empty() {
        eprintln!(
            "WARNING: {} asset(s) failed to download. Continuing anyway...",
            failed.len()
        );
    }

    Ok(())
}

async fn extract_natives(
    shared_dir: &Path,
    libraries: &[Library],
    natives_dir: &Path,
) -> Result<()> {
    let os_name = if cfg!(target_os = "windows") { "windows" } else if cfg!(target_os = "macos") { "osx" } else { "linux" };
    let natives_ext = match os_name {
        "windows" => "dll",
        "osx" => "dylib",
        _ => "so",
    };

    fs::create_dir_all(natives_dir)?;

    for lib in libraries {
        if let Some(natives) = &lib.natives {
            if let Some(classifier) = natives.get(os_name) {
                if let Some(classifiers) = &lib.downloads.classifiers {
                    if let Some(artifact) = classifiers.get(classifier) {
                        if let Some(path) = &artifact.path {
                            let lib_path = shared_dir.join("libraries").join(path);
                            if lib_path.exists() {
                                eprintln!("Extracting natives from: {}", lib_path.display());
                                let file = File::open(&lib_path)?;
                                let mut archive = ZipArchive::new(file)?;
                                for i in 0..archive.len() {
                                    let mut file = archive.by_index(i)?;
                                    let name = file.name().to_string();
                                    let file_name = Path::new(&name).file_name().unwrap_or_default();
                                    let file_name_str = file_name.to_string_lossy();
                                    if file_name_str.ends_with(&format!(".{}", natives_ext)) {
                                        let dest = natives_dir.join(&*file_name_str);
                                        let mut data = Vec::new();
                                        file.read_to_end(&mut data)?;
                                        fs::write(&dest, &data)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn find_java_executable(root: &Path, exe_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == exe_name {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn move_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    if let Err(e) = fs::rename(src, dst) {
        eprintln!("Rename failed ({}), falling back to recursive copy", e);
        fn copy_recursive(from: &Path, to: &Path) -> Result<()> {
            if !to.exists() {
                fs::create_dir_all(to)?;
            }
            for entry in fs::read_dir(from)? {
                let entry = entry?;
                let src_path = entry.path();
                let dst_path = to.join(entry.file_name());
                if src_path.is_dir() {
                    copy_recursive(&src_path, &dst_path)?;
                } else {
                    fs::copy(&src_path, &dst_path)?;
                }
            }
            Ok(())
        }
        copy_recursive(src, dst)?;
        fs::remove_dir_all(src)?;
    }
    Ok(())
}

async fn ensure_java(shared_dir: &Path, channel: &Channel<InstallProgress>) -> Result<PathBuf> {
    let java_dir = shared_dir.join("runtime").join("java");
    let java_exe = if cfg!(windows) {
        java_dir.join("bin").join("javaw.exe")
    } else {
        java_dir.join("bin").join("java")
    };

    if java_exe.exists() {
        let output = tokio::process::Command::new(&java_exe)
            .arg("-version")
            .output()
            .await?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains(&format!("{}", JAVA_MAJOR)) {
            eprintln!("Java already installed at: {}", java_exe.display());
            return Ok(java_exe);
        }
    }

    let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
    let arch = if cfg!(target_arch = "x86_64") { "x64" } else { "aarch64" };
    let ext = if cfg!(windows) { "zip" } else if cfg!(target_os = "macos") { "tar.gz" } else { "tar.gz" };

    let url = format! {
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
        JAVA_MAJOR, os, arch
    };

    let temp_dir = tempfile::tempdir()?;
    let download_path = temp_dir.path().join(format!("java.{}", ext));
    eprintln!("Downloading Java to: {}", download_path.display());

    send_progress(channel, "Downloading Java", 0, 1, 0.0, "Downloading Java 25...").await;
    download_file(
        &reqwest::Client::new(),
        &url,
        &download_path,
        None,
        Some(channel),
        "Downloading Java",
        "Java 25"
    ).await?;

    if !is_file_valid(&download_path) {
        return Err(anyhow!("Java download failed: file is empty or missing at {}", download_path.display()));
    }

    let java_temp = temp_dir.path().join("java_extracted");
    fs::create_dir_all(&java_temp)?;

    if ext == "zip" {
        eprintln!("Extracting Java ZIP from: {}", download_path.display());
        let file = File::open(&download_path)
            .with_context(|| format!("Failed to open Java archive at '{}'", download_path.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();
            let components: Vec<&str> = name.split('/').collect();
            let sanitized_components: Vec<String> = components
                .iter()
                .map(|c| sanitize_component(c))
                .collect();
            let relative_path: PathBuf = sanitized_components.iter().collect();
            let dest = java_temp.join(&relative_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            if file.is_file() {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                fs::write(&dest, &data)?;
            }
        }
    } else {
        eprintln!("Extracting Java TAR from: {}", download_path.display());
        let output = tokio::process::Command::new("tar")
            .arg("-xzf")
            .arg(&download_path)
            .arg("-C")
            .arg(&java_temp)
            .output()
            .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to extract Java: {}", stderr));
        }
    }

    let exe_name = if cfg!(windows) { "javaw.exe" } else { "java" };
    let found_exe = find_java_executable(&java_temp, exe_name)
        .ok_or_else(|| anyhow!("Java executable '{}' not found after extraction", exe_name))?;

    eprintln!("Found Java executable: {}", found_exe.display());

    let runtime_root = found_exe
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow!("Could not determine runtime root directory"))?;

    eprintln!("Runtime root directory: {}", runtime_root.display());

    move_dir(runtime_root, &java_dir)?;

    let final_exe = if cfg!(windows) {
        java_dir.join("bin").join("javaw.exe")
    } else {
        java_dir.join("bin").join("java")
    };

    if !final_exe.exists() {
        return Err(anyhow!("Java executable not found after move at {}", final_exe.display()));
    }

    eprintln!("Java installed successfully at: {}", final_exe.display());
    Ok(final_exe)
}

async fn get_java_executable(shared_dir: &Path, override_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path_str) = override_path {
        if !path_str.is_empty() {
            let path = PathBuf::from(path_str);
            if path.exists() {
                eprintln!("Using user-provided Java: {}", path.display());
                return Ok(path);
            } else {
                eprintln!("User-provided Java path does not exist: {}, falling back.", path_str);
            }
        }
    }

    let java_dir = shared_dir.join("runtime").join("java");
    let exe = if cfg!(windows) {
        java_dir.join("bin").join("javaw.exe")
    } else {
        java_dir.join("bin").join("java")
    };
    if exe.exists() {
        eprintln!("Using bundled Java: {}", exe.display());
        return Ok(exe);
    }

    if let Some(path) = detect_java_25_system().await {
        eprintln!("Using system Java: {}", path.display());
        return Ok(path);
    }

    Err(anyhow!("Java 25 not found"))
}

async fn detect_java_25_system() -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![
            r"C:\Program Files\Java\jdk-25\bin\javaw.exe",
            r"C:\Program Files\Java\jdk-21\bin\javaw.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-25\bin\javaw.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Library/Java/JavaVirtualMachines/temurin-25.jdk/Contents/Home/bin/java",
            "/usr/bin/java",
        ]
    } else {
        vec!["/usr/lib/jvm/java-25-openjdk/bin/java", "/usr/bin/java"]
    };

    for path in candidates {
        if std::path::Path::new(path).exists() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

#[derive(Debug, Serialize)]
struct JavaInstallation {
    path: String,
    version: String,
    major: u32,
    vendor: String,
}

fn extract_java_major(stderr: &str) -> u32 {
    let first_line = stderr.lines().next().unwrap_or("");
    if let Some(start) = first_line.find("version \"") {
        let ver_part = &first_line[start + 9..];
        if let Some(end) = ver_part.find('"') {
            let ver_str = &ver_part[..end];
            if ver_str.starts_with("1.8") {
                return 8;
            }
            if let Some(major_str) = ver_str.split('.').next() {
                if let Ok(major) = major_str.parse::<u32>() {
                    return major;
                }
            }
        }
    }
    for word in first_line.split_whitespace() {
        if let Ok(n) = word.parse::<u32>() {
            if n >= 8 && n <= 30 {
                return n;
            }
        }
    }
    0
}

fn detect_java_vendor(path: &Path, version_output: &str) -> String {
    let path_lower = path.to_string_lossy().to_lowercase();
    let out_lower = version_output.to_lowercase();
    
    if path_lower.contains("graalvm") || out_lower.contains("graalvm") {
        "GraalVM".into()
    } else if path_lower.contains("temurin") || path_lower.contains("adoptium") 
              || out_lower.contains("temurin") || out_lower.contains("adoptium") || out_lower.contains("eclipse") {
        "Eclipse Adoptium".into()
    } else if path_lower.contains("corretto") || out_lower.contains("corretto") || out_lower.contains("amazon") {
        "Amazon Corretto".into()
    } else if path_lower.contains("zulu") || out_lower.contains("zulu") || out_lower.contains("azul") {
        "Azul Zulu".into()
    } else if path_lower.contains("liberica") || path_lower.contains("bellsoft") || out_lower.contains("liberica") {
        "BellSoft Liberica".into()
    } else if path_lower.contains("microsoft") || out_lower.contains("microsoft") {
        "Microsoft".into()
    } else if path_lower.contains("oracle") || out_lower.contains("oracle") {
        "Oracle".into()
    } else if path_lower.contains("semeru") || out_lower.contains("semeru") || out_lower.contains("ibm") {
        "IBM Semeru".into()
    } else if path_lower.contains("adoptopenjdk") || out_lower.contains("adoptopenjdk") {
        "AdoptOpenJDK".into()
    } else if path_lower.contains("redhat") || out_lower.contains("red hat") {
        "Red Hat".into()
    } else if path_lower.contains("sapmachine") || out_lower.contains("sap") {
        "SAP".into()
    } else if path_lower.contains("jdk") || out_lower.contains("openjdk") {
        "OpenJDK".into()
    } else {
        "Unknown".into()
    }
}

fn try_add_java(path: &Path, result: &mut Vec<JavaInstallation>, seen: &mut HashSet<String>) {
    let Ok(canonical) = fs::canonicalize(path) else { return; };
    
    // FIX: strip the \\?\ prefix that canonicalize adds on Windows
    let path_str_raw = canonical.to_string_lossy().to_string();
    let path_str = path_str_raw
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str_raw)
        .to_string();
    
    if !seen.insert(path_str.clone()) { return; }
    
    let Ok(output) = std::process::Command::new(&canonical)
        .args(["-version"])
        .output() else { return; };
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = stderr.lines().next().unwrap_or("Unknown").to_string();
    let major = extract_java_major(&stderr);
    let vendor = detect_java_vendor(&canonical, &stderr);
    
    if major > 0 {
        result.push(JavaInstallation {
            path: path_str,
            version,
            major,
            vendor,
        });
    }
}

fn find_java_in_dir(
    dir: &Path,
    exe_name: &str,
    result: &mut Vec<JavaInstallation>,
    seen: &mut HashSet<String>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth { return; }
    
    let Ok(entries) = fs::read_dir(dir) else { return; };
    
    for entry in entries.flatten() {
        let path = entry.path();
        
        if path.is_dir() {
            if cfg!(target_os = "macos") {
                if path.extension().and_then(|s| s.to_str()) == Some("jdk") {
                    let candidate = path.join("Contents").join("Home").join("bin").join(exe_name);
                    if candidate.exists() {
                        try_add_java(&candidate, result, seen);
                        continue;
                    }
                }
            }
            
            if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(false) {
                if let Ok(real) = fs::canonicalize(&path) {
                    if real == path { continue; }
                }
            }
            
            find_java_in_dir(&path, exe_name, result, seen, depth + 1, max_depth);
        } else if path.is_file() {
            if path.file_name().and_then(|s| s.to_str()) == Some(exe_name) {
                try_add_java(&path, result, seen);
            }
        }
    }
}

#[tauri::command]
async fn get_bundled_java_path(app: AppHandle) -> Result<Option<String>, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let shared_dir = get_shared_dir(&app_dir);
    let java_dir = shared_dir.join("runtime").join("java");
    let exe = if cfg!(windows) {
        java_dir.join("bin").join("javaw.exe")
    } else {
        java_dir.join("bin").join("java")
    };
    if exe.exists() {
        Ok(Some(exe.to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
async fn list_java_installations(app: AppHandle) -> Result<Vec<JavaInstallation>, String> {
    let mut result = Vec::new();
    let platform = std::env::consts::OS;
    let exe_name = if cfg!(windows) { "javaw.exe" } else { "java" };
    let mut seen = HashSet::new();

    if let Ok(app_dir) = get_app_dir(&app) {
        let shared_dir = get_shared_dir(&app_dir);
        let bundled = if cfg!(windows) {
            shared_dir.join("runtime").join("java").join("bin").join("javaw.exe")
        } else {
            shared_dir.join("runtime").join("java").join("bin").join("java")
        };
        if bundled.exists() {
            if let Ok(output) = std::process::Command::new(&bundled)
                .args(["-version"])
                .output()
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let version = stderr.lines().next().unwrap_or("Bundled Java").to_string();
                let major = extract_java_major(&stderr);
                let vendor = detect_java_vendor(&bundled, &stderr);
                result.push(JavaInstallation {
                    path: bundled.to_string_lossy().to_string(),
                    version: if version.is_empty() { "Bundled Java".to_string() } else { version },
                    major,
                    vendor,
                });
                seen.insert(bundled.to_string_lossy().to_string());
            }
        }
    }

    let mut search_roots: Vec<PathBuf> = Vec::new();

    if platform == "windows" {
        search_roots.push(PathBuf::from(r"C:\Program Files\Java"));
        search_roots.push(PathBuf::from(r"C:\Program Files (x86)\Java"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Eclipse Adoptium"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Microsoft"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Amazon Corretto"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Zulu"));
        search_roots.push(PathBuf::from(r"C:\Program Files\BellSoft"));
        search_roots.push(PathBuf::from(r"C:\Program Files\AdoptOpenJDK"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Semeru"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Temurin"));
        search_roots.push(PathBuf::from(r"C:\Program Files\Oracle"));
    } else if platform == "macos" {
        search_roots.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
        search_roots.push(PathBuf::from("/System/Library/Java/JavaVirtualMachines"));
        search_roots.push(PathBuf::from("/usr/local/opt"));
        search_roots.push(PathBuf::from("/opt/homebrew/opt"));
    } else {
        search_roots.push(PathBuf::from("/usr/lib/jvm"));
        search_roots.push(PathBuf::from("/usr/java"));
        search_roots.push(PathBuf::from("/opt/java"));
        search_roots.push(PathBuf::from("/usr/local/java"));
        search_roots.push(PathBuf::from("/snap"));
    }

    if let Ok(jh) = std::env::var("JAVA_HOME") {
        let jh_path = PathBuf::from(&jh);
        if jh_path.exists() {
            if let Some(parent) = jh_path.parent() {
                if parent != jh_path {
                    search_roots.push(parent.to_path_buf());
                }
            }
            search_roots.push(jh_path);
        }
    }

    for root in search_roots {
        if !root.exists() { continue; }
        find_java_in_dir(&root, exe_name, &mut result, &mut seen, 0, 3);
    }

    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for p in path_var.split(sep) {
            let path_dir = PathBuf::from(p);
            let exe = path_dir.join(exe_name);
            if exe.exists() {
                try_add_java(&exe, &mut result, &mut seen);
            }
        }
    }

    result.sort_by(|a, b| b.major.cmp(&a.major));
    result.dedup_by(|a, b| a.path == b.path);
    Ok(result)
}

#[tauri::command]
async fn install_recommended_java(
    app: AppHandle,
    progress_channel: Channel<InstallProgress>,
) -> Result<String, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let shared_dir = get_shared_dir(&app_dir);

    let java_path = ensure_java(&shared_dir, &progress_channel)
        .await
        .map_err(|e| e.to_string())?;

    Ok(java_path.to_string_lossy().to_string())
}

// -----------------------------------------------
// PHASE 1: EXTRACT FORGE'S EMBEDDED LIBRARIES
// -----------------------------------------------

async fn extract_embedded_libraries(
    installer_path: &Path,
    libs_dir: &Path,
) -> Result<()> {
    eprintln!("Extracting Forge's embedded libraries...");
    
    let file = File::open(installer_path)
        .context("Could not open the Forge installer")?;
    let mut zip = ZipArchive::new(file)?;
    
    let mut extracted_count = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        
        if !name.starts_with("maven/") || name.ends_with('/') {
            continue;
        }
        
        let rel_path = &name[6..];
        let dest = libs_dir.join(rel_path);
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        
        fs::write(&dest, &data)?;
        
        extracted_count += 1;
        eprintln!("  OK {}", rel_path);
    }
    
    eprintln!("{} embedded libraries extracted", extracted_count);
    Ok(())
}

// -----------------------------------------------
// PHASE 2: DOWNLOAD LOG4J CONFIGURATION
// -----------------------------------------------

async fn download_log4j_config(
    client: &reqwest::Client,
    mojang_json: &Value,
    shared_dir: &Path,
    channel: &Channel<InstallProgress>,
) -> Result<Option<String>> {
    let logging = match mojang_json.get("logging") {
        Some(Value::Object(log)) => log,
        _ => return Ok(None),
    };
    
    let client_logging = match logging.get("client") {
        Some(Value::Object(cl)) => cl,
        _ => return Ok(None),
    };
    
    let file_info = match client_logging.get("file") {
        Some(Value::Object(f)) => f,
        _ => return Ok(None),
    };
    
    let log_url = file_info.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing logging file url"))?;
    
    let log_sha1 = file_info.get("sha1")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing logging file sha1"))?;
    
    let log_config_dir = shared_dir.join("log_configs");
    fs::create_dir_all(&log_config_dir)?;
    
    let dest = log_config_dir.join("log4j2.xml");
    
    if is_file_valid(&dest) {
        let actual = compute_sha1(&dest)?;
        if actual == log_sha1 {
            eprintln!("Log4j config already downloaded: {}", dest.display());
            return Ok(Some(dest.to_string_lossy().to_string()));
        }
    }
    
    eprintln!("Downloading Log4j config...");
    send_progress(channel, "Downloading Log4j", 0, 1, 0.0, "log4j2.xml").await;
    
    download_file(
        client,
        log_url,
        &dest,
        Some(log_sha1),
        Some(channel),
        "Downloading Log4j",
        "log4j2.xml"
    ).await?;
    
    eprintln!("Log4j downloaded: {}", dest.display());
    Ok(Some(dest.to_string_lossy().to_string()))
}

// -----------------------------------------------
// PROCESS FORGE
// -----------------------------------------------

fn replace_placeholders(
    arg: &str,
    classpath: &str,
    libraries_dir: &Path,
    natives_dir: &Path,
    sep: &str,
    minecraft_version: &str,
    forge_version: &str,
    username: &str,
    uuid: &str,
    access_token: &str,
    user_type: &str,
) -> String {
    let mut result = arg.to_string();
    
    if result.contains("${classpath}") {
        result = result.replace("${classpath}", classpath);
    }
    if result.contains("${classpath_separator}") {
        result = result.replace("${classpath_separator}", sep);
    }
    if result.contains("${library_directory}") {
        result = result.replace("${library_directory}", &libraries_dir.to_string_lossy());
    }
    if result.contains("${natives_directory}") {
        result = result.replace("${natives_directory}", &natives_dir.to_string_lossy());
    }
    if result.contains("${minecraft_version}") {
        result = result.replace("${minecraft_version}", minecraft_version);
    }
    if result.contains("${forge_version}") {
        result = result.replace("${forge_version}", forge_version);
    }
    if result.contains("${version_name}") {
        result = result.replace("${version_name}", minecraft_version);
    }
    if result.contains("${auth_player_name}") {
        result = result.replace("${auth_player_name}", username);
    }
    if result.contains("${auth_uuid}") {
        result = result.replace("${auth_uuid}", uuid);
    }
    if result.contains("${auth_access_token}") {
        result = result.replace("${auth_access_token}", access_token);
    }
    if result.contains("${auth_session}") {
        result = result.replace("${auth_session}", access_token);
    }
    if result.contains("${user_type}") {
        result = result.replace("${user_type}", user_type);
    }
    if result.contains("${user_properties}") {
        result = result.replace("${user_properties}", "{}");
    }
    if result.contains("${assets_root}") {
        result = result.replace("${assets_root}", &libraries_dir.join("assets").to_string_lossy());
    }
    if result.contains("${game_assets}") {
        result = result.replace("${game_assets}", &libraries_dir.join("assets").to_string_lossy());
    }
    if result.contains("${launcher_name}") {
        result = result.replace("${launcher_name}", "Sparkle");
    }
    if result.contains("${launcher_version}") {
        result = result.replace("${launcher_version}", "0.9.0");
    }
    
    result
}

fn resolve_maven_coordinate(coord: &str, libraries: &[Library]) -> Result<PathBuf> {
    for lib in libraries {
        if lib.name == coord {
            if let Some(artifact) = &lib.downloads.artifact {
                if let Some(path) = &artifact.path {
                    return Ok(PathBuf::from(path));
                }
            }
        }
    }

    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 {
        return Err(anyhow!("Invalid Maven coordinate: {}", coord));
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    
    let mut classifier = String::new();
    let mut extension = "jar".to_string();
    
    if parts.len() > 3 {
        let rest_joined = parts[3..].join(":");
        if let Some(at_pos) = rest_joined.find('@') {
            let (before, after) = rest_joined.split_at(at_pos);
            extension = after.trim_start_matches('@').to_string();
            if !before.is_empty() {
                classifier = format!("-{}", before);
            }
        } else if !rest_joined.is_empty() {
            classifier = format!("-{}", rest_joined);
        }
    }
    
    if parts.len() == 3 && parts[0] == "de.oceanlabs.mcp" && parts[1] == "mcp_config" {
        let rel_path = format! {
            "de/oceanlabs/mcp/mcp_config/{0}/mcp_config-{0}.zip",
            version
        };
        return Ok(PathBuf::from(rel_path));
    }

    let filename = format!("{}-{}{}.{}", artifact, version, classifier, extension);
    
    let relative_path = PathBuf::from(group)
        .join(artifact)
        .join(version)
        .join(filename);

    Ok(relative_path)
}

fn resolve_maven_coordinate_with_extraction(
    arg: &str,
    libraries: &[Library],
    libs_dir: &Path,
) -> Result<String> {
    if arg.starts_with('[') && arg.ends_with(']') {
        let inner = &arg[1..arg.len() - 1];
        let rel_path = resolve_maven_coordinate(inner, libraries)?;
        let full_path = libs_dir.join(&rel_path);

        if full_path.exists() {
            return Ok(full_path.to_string_lossy().to_string());
        }

        if let (Some(parent), Some(stem)) = (full_path.parent(), full_path.file_stem()) {
            let stem = stem.to_string_lossy().to_string();
            for ext in ["jar", "zip"] {
                let candidate = parent.join(format!("{}.{}", stem, ext));
                if candidate.exists() {
                    return Ok(candidate.to_string_lossy().to_string());
                }
            }
        }

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for '{}'", full_path.display()))?;
        }
        Ok(full_path.to_string_lossy().to_string())
    } else {
        Ok(arg.to_string())
    }
}

fn extract_jar_resource(jar_path: &Path, internal_path: &str, dest_dir: &Path) -> Result<PathBuf> {
    let entry_name = internal_path.trim_start_matches(['/', '\\']);
    let file = File::open(jar_path)
        .with_context(|| format!("Failed to open jar '{}' to extract resource", jar_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    let mut zip_file = archive.by_name(entry_name).with_context(|| {
        format!("Resource '{}' not found inside '{}'", entry_name, jar_path.display())
    })?;

    let mut data = Vec::new();
    zip_file.read_to_end(&mut data)?;

    let dest_path = dest_dir.join(entry_name.replace('/', std::path::MAIN_SEPARATOR_STR));
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for '{}'", dest_path.display()))?;
    }
    fs::write(&dest_path, &data)
        .with_context(|| format!("Failed to write extracted resource to '{}'", dest_path.display()))?;

    Ok(dest_path)
}

fn extract_processor_args(args: &Value) -> Vec<String> {
    match args {
        Value::Array(arr) => {
            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
        }
        Value::Object(map) => {
            if let Some(client_val) = map.get("client") {
                if let Some(arr) = client_val.as_array() {
                    return arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn get_main_class_from_jar(jar_path: &Path) -> Result<String> {
    let file = File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();
        if name == "META-INF/MANIFEST.MF" {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("Main-Class: ") {
                    return Ok(rest.trim().to_string());
                }
            }
        }
    }
    Err(anyhow!("Main-Class not found in MANIFEST.MF of {}", jar_path.display()))
}

fn substitute_variables(arg: &str, vars: &HashMap<String, String>) -> String {
    let mut result = arg.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{}}}", key);
        if result.contains(&placeholder) {
            result = result.replace(&placeholder, value);
        }
    }
    result
}

async fn process_forge_installation(
    shared_dir: &Path,
    installer_path: &Path,
    forge_dir: &Path,
    java_path: &Path,
    channel: &Channel<InstallProgress>,
) -> Result<()> {
    extract_forge_metadata(installer_path, forge_dir)?;

    let install_profile_path = forge_dir.join("install_profile.json");
    if !is_json_valid(&install_profile_path) {
        return Err(anyhow!("install_profile.json not found or invalid"));
    }

    let install_profile: InstallProfile = serde_json::from_reader(File::open(&install_profile_path)?)?;
    eprintln!("Found {} processors in install_profile", install_profile.processors.len());

    let libs_dir = shared_dir.join("libraries");
    fs::create_dir_all(&libs_dir)?;
    
    eprintln!("Extracting Forge's embedded libraries...");
    send_progress(channel, "Extracting libraries", 0, 1, 0.0, "Extracting embedded libraries...").await;
    extract_embedded_libraries(installer_path, &libs_dir).await?;
    eprintln!("Embedded libraries extracted");

    let mut variables = HashMap::new();
    variables.insert("ROOT".to_string(), shared_dir.to_string_lossy().to_string());
    variables.insert("SIDE".to_string(), "client".to_string());
    variables.insert("INSTALLER".to_string(), installer_path.to_string_lossy().to_string());
    let minecraft_jar_path = shared_dir.join("versions").join(MINECRAFT_VERSION).join(format!("{}.jar", MINECRAFT_VERSION));
    if !is_file_valid(&minecraft_jar_path) {
        return Err(anyhow!("Minecraft client jar not found or empty at {}", minecraft_jar_path.display()));
    }
    variables.insert("MINECRAFT_JAR".to_string(), minecraft_jar_path.to_string_lossy().to_string());
    for (key, entry) in &install_profile.data {
        if let Some(client_val) = &entry.client {
            let resolved = substitute_variables(client_val, &variables);
            let final_value = if resolved.starts_with('/') || resolved.starts_with('\\') {
                let extracted = extract_jar_resource(installer_path, &resolved, forge_dir)
                    .with_context(|| format!("Failed to extract embedded resource '{}' for variable '{}'", resolved, key))?;
                extracted.to_string_lossy().to_string()
            } else {
                resolved
            };
            variables.insert(key.clone(), final_value);
        }
    }

    eprintln!("Variables map: {:#?}", variables);

    let mut processor_libs = install_profile.libraries.clone();
    processor_libs.retain(library_applies);
    processor_libs = deduplicate_libraries(processor_libs);

    if !processor_libs.is_empty() {
        eprintln!("Downloading {} processor libraries", processor_libs.len());
        send_progress(channel, "Downloading processor libraries", 0, processor_libs.len() as u64, 0.0, "Preparing...").await;
        download_libraries(
            &reqwest::Client::new(),
            shared_dir,
            &processor_libs,
            channel,
        ).await?;
    }

    let libs_dir = shared_dir.join("libraries");
    let sep = if cfg!(windows) { ";" } else { ":" };
    let natives_dir = shared_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;

    let mut processors_to_run = Vec::new();
    for (idx, processor) in install_profile.processors.iter().enumerate() {
        let sides = &processor.sides;
        let jar = &processor.jar;
        let args_str = format!("{:?}", processor.args);

        if !sides.is_empty() && sides.contains(&"server".to_string()) && !sides.contains(&"client".to_string()) {
            eprintln!("Skipping processor {} (sides: {:?}) - server-only", idx + 1, sides);
            continue;
        }

        if sides.is_empty() {
            if jar.contains("server") || args_str.contains("BUNDLER_EXTRACT") {
                eprintln!("Skipping processor {} (no sides, but seems server-related)", idx + 1);
                continue;
            }
        }

        if sides.is_empty() || sides.contains(&"client".to_string()) {
            eprintln!("Will run processor {}", idx + 1);
            processors_to_run.push((idx, processor));
        }
    }

    eprintln!("Running {} out of {} processors", processors_to_run.len(), install_profile.processors.len());

    for (idx, processor) in processors_to_run {
        let progress = (idx as f32 / install_profile.processors.len() as f32) * 100.0;
        send_progress(
            channel,
            "Processing Forge installation",
            idx as u64 + 1,
            install_profile.processors.len() as u64,
            progress,
            &format!("Running processor {}/{}", idx + 1, install_profile.processors.len()),
        ).await;

        eprintln!("Running processor {}: {}", idx + 1, processor.jar);

        let processor_jar_rel = resolve_maven_coordinate(&processor.jar, &install_profile.libraries)?;
        let processor_jar_path = libs_dir.join(&processor_jar_rel);
        if !processor_jar_path.exists() {
            return Err(anyhow!("Processor jar not found: {}", processor_jar_path.display()));
        }

        let main_class = get_main_class_from_jar(&processor_jar_path)?;
        eprintln!("Processor main class: {}", main_class);

        let mut cp_paths = Vec::new();
        cp_paths.push(processor_jar_path.clone());
        for coord in &processor.classpath {
            let rel_path = resolve_maven_coordinate(coord, &install_profile.libraries)?;
            let full_path = libs_dir.join(&rel_path);
            if full_path.exists() {
                cp_paths.push(full_path);
            } else {
                eprintln!("Warning: classpath entry {} not found at {}", coord, full_path.display());
            }
        }
        let cp_str = cp_paths.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(sep);

        let args_vec = extract_processor_args(&processor.args);

        let mut resolved_args = Vec::new();
        for arg in &args_vec {
            let substituted = substitute_variables(arg, &variables);
            let replaced = replace_placeholders(
                &substituted,
                &cp_str,
                &libs_dir,
                &natives_dir,
                sep,
                MINECRAFT_VERSION,
                FORGE_VERSION,
                "Player",
                "00000000-0000-0000-0000-000000000000",
                "offline",
                "legacy",
            );
            let resolved = resolve_maven_coordinate_with_extraction(
                &replaced,
                &install_profile.libraries,
                &libs_dir,
            )?;
            resolved_args.push(resolved);
        }

        let mut args = Vec::new();
        args.push("-cp".to_string());
        args.push(cp_str.clone());
        args.push(main_class);
        for arg in resolved_args {
            args.push(arg);
        }

        eprintln!("Executing: {} {}", java_path.display(), args.join(" "));
        let output = tokio::process::Command::new(java_path)
            .args(&args)
            .output()
            .await?;

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        eprintln!("Processor stdout:\n{}", stdout_str);
        eprintln!("Processor stderr:\n{}", stderr_str);

        if !output.status.success() {
            eprintln!("Resolved args: {:?}", args);
            return Err(anyhow!("Processor {} failed with exit code: {:?}\nstderr: {}", idx + 1, output.status.code(), stderr_str));
        }

        eprintln!("Processor {} completed successfully", idx + 1);
    }

    let forge_client_jar = libs_dir.join("net").join("minecraftforge").join("forge")
        .join(format!("{}-{}", MINECRAFT_VERSION, FORGE_VERSION))
        .join(format!("forge-{}-{}-client.jar", MINECRAFT_VERSION, FORGE_VERSION));

    if forge_client_jar.exists() {
        eprintln!("Forge client jar generated: {}", forge_client_jar.display());
    } else {
        eprintln!("Warning: forge-client.jar not found at expected location.");
        let alt_forge_client = forge_dir.join(format!("forge-{}-{}-client.jar", MINECRAFT_VERSION, FORGE_VERSION));
        if alt_forge_client.exists() {
            eprintln!("Found forge-client.jar in forge_dir: {}", alt_forge_client.display());
            let dest = libs_dir.join("net").join("minecraftforge").join("forge")
                .join(format!("{}-{}", MINECRAFT_VERSION, FORGE_VERSION));
            fs::create_dir_all(&dest)?;
            fs::copy(&alt_forge_client, dest.join(alt_forge_client.file_name().unwrap()))?;
        }
    }

    let version_json_path = forge_dir.join("version.json");
    if !is_json_valid(&version_json_path) {
        return Err(anyhow!("version.json not generated after processors"));
    }

    eprintln!("Forge installation via processors completed successfully.");
    Ok(())
}

// -----------------------------------------------
// GAME LAUNCH
// -----------------------------------------------

async fn launch_game_impl(
    app: &AppHandle,
    progress_channel: &Channel<InstallProgress>,
    shared_dir: &Path,
    instance_dir: &Path,
    account: Option<Account>,
    max_ram: u64,
    java_args_extra: &str,
    java_path_override: &str,
) -> Result<()> {
    let version = MINECRAFT_VERSION;
    let forge_ver = FORGE_VERSION;
    let forge_dir = shared_dir.join("versions").join(format!("{}-forge-{}", version, forge_ver));
    let libs_dir = shared_dir.join("libraries");
    let forge_version_json_path = forge_dir.join("version.json");

    eprintln!("Forge dir: {}", forge_dir.display());
    eprintln!("Forge dir exists: {}", forge_dir.exists());
    eprintln!("version.json exists: {}", forge_version_json_path.exists());

    let mojang_version_json_path = shared_dir.join("versions").join(version).join(format!("{}.json", version));
    if !is_json_valid(&mojang_version_json_path) {
        return Err(anyhow!("Mojang version.json is missing or corrupt at {}", mojang_version_json_path.display()));
    }
    let mojang_json: Value = serde_json::from_reader(File::open(&mojang_version_json_path)?)?;
    let mojang_main_class = mojang_json["mainClass"].as_str()
        .ok_or_else(|| anyhow!("Missing mainClass in Mojang version.json"))?.to_string();
    let asset_index_id = mojang_json["assetIndex"]["id"].as_str()
        .ok_or_else(|| anyhow!("Missing assetIndex.id in Mojang version.json"))?.to_string();

    let mojang_libs: Vec<Library> = if let Some(libs) = mojang_json["libraries"].as_array() {
        libs.iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .filter(library_applies)
            .collect()
    } else {
        Vec::new()
    };

    let mut libraries = mojang_libs;

    let use_forge = is_json_valid(&forge_version_json_path);
    let mut forge_main_class: Option<String> = None;
    let mut forge_jvm_args: Vec<String> = Vec::new();
    let mut forge_game_args: Vec<String> = Vec::new();

    if use_forge {
        eprintln!("Forge detected, reading from version.json");
        let forge_json: Value = serde_json::from_reader(File::open(&forge_version_json_path)?)?;
        forge_main_class = forge_json["mainClass"].as_str().map(|s| s.to_string());
        if let Some(args) = forge_json["arguments"].as_object() {
            if let Some(jvm) = args.get("jvm").and_then(|v| v.as_array()) {
                for arg in jvm {
                    if let Some(s) = arg.as_str() {
                        forge_jvm_args.push(s.to_string());
                    }
                }
            }
            if let Some(game) = args.get("game").and_then(|v| v.as_array()) {
                for arg in game {
                    if let Some(s) = arg.as_str() {
                        forge_game_args.push(s.to_string());
                    }
                }
            }
        } else if let Some(game_args_str) = forge_json["minecraftArguments"].as_str() {
            for part in game_args_str.split_whitespace() {
                forge_game_args.push(part.to_string());
            }
        }
        if let Some(libs) = forge_json["libraries"].as_array() {
            let forge_libs: Vec<Library> = libs.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .filter(library_applies)
                .collect();
            libraries.extend(forge_libs);
        }
        libraries = deduplicate_libraries(libraries);
    } else {
        eprintln!("No Forge detected, using vanilla main class");
    }

    let mut classpath = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for lib in &libraries {
        if let Some(artifact) = &lib.downloads.artifact {
            if let Some(path) = &artifact.path {
                let full_path = libs_dir.join(path);
                if full_path.exists() && seen_paths.insert(full_path.clone()) {
                    classpath.push(full_path);
                }
            }
        }
        if let Some(classifiers) = &lib.downloads.classifiers {
            for (_, artifact) in classifiers {
                if let Some(path) = &artifact.path {
                    let full_path = libs_dir.join(path);
                    if full_path.exists() && seen_paths.insert(full_path.clone()) {
                        classpath.push(full_path);
                    }
                }
            }
        }
    }

    let client_jar = shared_dir.join("versions").join(version).join(format!("{}.jar", version));
    if !is_file_valid(&client_jar) {
        return Err(anyhow!("Minecraft client jar is missing or empty at {}", client_jar.display()));
    }
    if seen_paths.insert(client_jar.clone()) {
        classpath.push(client_jar);
    }

    if use_forge {
        let forge_client_jars = [
            format!("forge-{}-{}-client.jar", version, forge_ver),
            format!("forge-{}-{}-universal.jar", version, forge_ver),
        ];
        for jar_name in &forge_client_jars {
            let forge_jar = forge_dir.join(jar_name);
            if forge_jar.exists() && seen_paths.insert(forge_jar.clone()) {
                classpath.push(forge_jar);
                break;
            }
            let alt_path = libs_dir.join("net").join("minecraftforge").join("forge")
                .join(format!("{}-{}", version, forge_ver))
                .join(jar_name);
            if alt_path.exists() && seen_paths.insert(alt_path.clone()) {
                classpath.push(alt_path);
                break;
            }
        }
        let client_dir = libs_dir.join("net").join("minecraft").join("client");
        if client_dir.exists() {
            for entry in fs::read_dir(&client_dir)?.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    for jar_name in ["client-srg.jar", "client-extra.jar"] {
                        let jar_path = path.join(jar_name);
                        if jar_path.exists() && seen_paths.insert(jar_path.clone()) {
                            classpath.push(jar_path);
                        }
                    }
                }
            }
        }
    }

    let sep = if cfg!(windows) { ";" } else { ":" };
    let cp_str = classpath.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>().join(sep);

    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(cp_str.as_bytes())?;
    let temp_path = temp_file.path().to_string_lossy().to_string();
    let cp_arg = format!("@{}", temp_path);

    let java_path = get_java_executable(shared_dir, Some(java_path_override)).await?;

    let min_ram = if max_ram > 512 { 512 } else { max_ram };
    let max_ram = max_ram.max(256);

    let natives_dir = instance_dir.join("natives");
    fs::create_dir_all(&natives_dir)?;

    let is_premium = account
        .as_ref()
        .map(|a| a.account_type == "microsoft" && !a.access_token.trim().is_empty())
        .unwrap_or(false);

    let (username, uuid, access_token, user_type): (String, String, String, String) = match &account {
        Some(acc) if is_premium => {
            eprintln!("Launching in PREMIUM mode as '{}'", acc.username);
            (acc.username.clone(), acc.uuid.clone(), acc.access_token.clone(), "msa".to_string())
        }
        Some(acc) => {
            eprintln!("Launching in OFFLINE mode as '{}'", acc.username);
            (acc.username.clone(), acc.uuid.clone(), "offline".to_string(), "legacy".to_string())
        }
        None => {
            eprintln!("Launching in OFFLINE mode as 'Player' (no session found)");
            (
                "Player".to_string(),
                "00000000-0000-0000-0000-000000000000".to_string(),
                "offline".to_string(),
                "legacy".to_string(),
            )
        }
    };

    let game_dir_str = instance_dir.to_string_lossy().to_string();
    let assets_dir_str = shared_dir.join("assets").to_string_lossy().to_string();
    let version_type = "release";

    let mut jvm_args: Vec<String> = Vec::new();
    jvm_args.push(format!("-Xms{}M", min_ram));
    jvm_args.push(format!("-Xmx{}M", max_ram));
    
    let log_configs_dir = shared_dir.join("log_configs");
    let log4j_file = log_configs_dir.join("log4j2.xml");
    
    eprintln!("Looking for Log4j at: {}", log_configs_dir.display());
    
    if log4j_file.exists() {
        let log4j_canonical = fs::canonicalize(&log4j_file)
            .unwrap_or_else(|_| log4j_file.clone())
            .to_string_lossy()
            .to_string();

        let log4j_canonical = log4j_canonical
            .strip_prefix(r"\\?\")
            .unwrap_or(&log4j_canonical)
            .to_string();
        
        let log4j_uri = if cfg!(windows) {
            let path_with_slashes = log4j_canonical.replace('\\', "/");
            format!("file:///{}", path_with_slashes)
        } else {
            format!("file://{}", log4j_canonical)
        };
        
        jvm_args.push("-Dlog4j2.formatMsgNoLookups=true".to_string());
        jvm_args.push(format!("-Dlog4j.configurationFile={}", log4j_uri));
        eprintln!("Log4j configured: {}", log4j_uri);
    } else {
        eprintln!("CRITICAL: Log4j config NOT found at {}", log_configs_dir.display());
        if let Ok(entries) = fs::read_dir(&log_configs_dir) {
            eprintln!("Files in log_configs:");
            for entry in entries.filter_map(Result::ok) {
                eprintln!("  - {}", entry.file_name().to_string_lossy());
            }
        } else {
            eprintln!("  logs directory DOES NOT EXIST");
        }
    }

    if !java_args_extra.trim().is_empty() {
        for arg in java_args_extra.split_whitespace() {
            if !arg.is_empty() {
                jvm_args.push(arg.to_string());
            }
        }
    }

    if use_forge && !forge_jvm_args.is_empty() {
        for arg in forge_jvm_args {
            let replaced = replace_placeholders(
                &arg,
                &cp_str,
                &libs_dir,
                &natives_dir,
                sep,
                version,
                forge_ver,
                &username,
                &uuid,
                &access_token,
                &user_type,
            );
            if replaced == "-cp" || replaced == "-classpath" {
                continue;
            }
            if !replaced.starts_with("-Djava.library.path") {
                jvm_args.push(replaced);
            }
        }
        if !jvm_args.iter().any(|a| a.starts_with("-Djava.library.path")) {
            jvm_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        }
        jvm_args.push("-cp".to_string());
        jvm_args.push(cp_arg.clone());
    } else {
        jvm_args.push("-XX:+UseG1GC".to_string());
        jvm_args.push("-XX:+ParallelRefProcEnabled".to_string());
        jvm_args.push("-XX:MaxGCPauseMillis=200".to_string());
        jvm_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
        jvm_args.push("-cp".to_string());
        jvm_args.push(cp_arg.clone());
    }

    let main_class = if use_forge {
        forge_main_class.unwrap_or_else(|| "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string())
    } else {
        mojang_main_class
    };

    let mut game_args: Vec<String> = Vec::new();
    if use_forge && !forge_game_args.is_empty() {
        for arg in forge_game_args {
            let replaced = replace_placeholders(
                &arg,
                &cp_str,
                &libs_dir,
                &natives_dir,
                sep,
                version,
                forge_ver,
                &username,
                &uuid,
                &access_token,
                &user_type,
            );
            game_args.push(replaced);
        }
    } else {
        game_args.push(format!("--username={}", username));
        game_args.push(format!("--uuid={}", uuid));
        game_args.push(format!("--accessToken={}", access_token));
        game_args.push(format!("--gameDir={}", game_dir_str));
        game_args.push(format!("--assetsDir={}", assets_dir_str));
        game_args.push(format!("--assetIndex={}", asset_index_id));
        game_args.push(format!("--version={}", version));
        game_args.push(format!("--versionType={}", version_type));
        game_args.push(format!("--userType={}", user_type));
    }

    if !game_args.iter().any(|a| a.starts_with("--username")) {
        game_args.push(format!("--username={}", username));
    }
    if !game_args.iter().any(|a| a.starts_with("--uuid")) {
        game_args.push(format!("--uuid={}", uuid));
    }
    if !game_args.iter().any(|a| a.starts_with("--accessToken")) {
        game_args.push(format!("--accessToken={}", access_token));
    }
    if !game_args.iter().any(|a| a.starts_with("--userType")) {
        game_args.push(format!("--userType={}", user_type));
    }
    if !game_args.iter().any(|a| a.starts_with("--gameDir")) {
        game_args.push(format!("--gameDir={}", game_dir_str));
    }
    if !game_args.iter().any(|a| a.starts_with("--assetsDir")) {
        game_args.push(format!("--assetsDir={}", assets_dir_str));
    }
    if !game_args.iter().any(|a| a.starts_with("--assetIndex")) {
        game_args.push(format!("--assetIndex={}", asset_index_id));
    }
    if !game_args.iter().any(|a| a.starts_with("--versionType")) {
        game_args.push(format!("--versionType={}", version_type));
    }
    if !game_args.iter().any(|a| a.starts_with("--version=") || a == "--version") {
        game_args.push(format!("--version={}", version));
    }

    let mut cmd = tokio::process::Command::new(&java_path);
    cmd.current_dir(instance_dir);
    cmd.args(&jvm_args);
    cmd.arg(&main_class);
    cmd.args(&game_args);

    eprintln!("Launch command: {:?}", cmd);
    eprintln!("Working dir: {}", instance_dir.display());
    eprintln!("Username: {}, UUID: {}, AccessToken: {}, UserType: {}", username, uuid, access_token, user_type);

    eprintln!("Launching Minecraft...");
    let mut child = cmd.spawn()?;

    if let Some(pid) = child.id() {
        if let Some(state) = app.try_state::<GameProcessState>() {
            *state.pid.lock().unwrap() = Some(pid);
        }
        // We notify the frontend that the game process has actually started (not just
        // that it's being prepared), so the button switches from "Launching..."
        // to "Stop".
        let _ = progress_channel.send(InstallProgress {
            stage: "launched".to_string(),
            current: 1,
            total: 1,
            percent: 100.0,
            detail: "Game process started".to_string(),
            speed: 0.0,
            eta: None,
        });
    }

    eprintln!("Waiting for Minecraft to start...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // --- Playtime tracking ---------------------------------------------
    // Instead of waiting for the whole session in one shot and only
    // recording playtime at the very end (which loses everything if the
    // launcher process itself is killed/crashes while the game is still
    // running), we checkpoint the elapsed time into instance.json every
    // ~60s. Worst case if the launcher dies mid-session: at most ~60s of
    // playtime is lost, instead of the entire session.
    let playtime_instance_path = get_instance_json_path(instance_dir);
    const PLAYTIME_CHECKPOINT: Duration = Duration::from_secs(60);
    let mut last_checkpoint = Instant::now();

    let wait_result = loop {
        tokio::select! {
            biased;
            result = child.wait() => {
                break result;
            }
            _ = tokio::time::sleep(PLAYTIME_CHECKPOINT) => {
                let elapsed = last_checkpoint.elapsed().as_secs();
                if let Err(e) = add_instance_playtime(&playtime_instance_path, elapsed) {
                    eprintln!("Could not save playtime checkpoint: {e}");
                }
                last_checkpoint = Instant::now();
            }
        }
    };

    // Final flush: whatever elapsed since the last checkpoint (or since
    // launch, if the session was shorter than one checkpoint interval).
    let final_elapsed = last_checkpoint.elapsed().as_secs();
    if let Err(e) = add_instance_playtime(&playtime_instance_path, final_elapsed) {
        eprintln!("Could not save final playtime: {e}");
    }

    let status = wait_result?;

    // If the process ended because the user clicked "Stop" from the
    // launcher, we don't treat it as a game failure.
    let was_stop_requested = if let Some(state) = app.try_state::<GameProcessState>() {
        *state.pid.lock().unwrap() = None;
        let mut stop_guard = state.stop_requested.lock().unwrap();
        let was_requested = *stop_guard;
        *stop_guard = false;
        was_requested
    } else {
        false
    };
    
    let latest_log = instance_dir.join("logs").join("latest.log");
    
    if latest_log.exists() {
        eprintln!("Log file generated: {}", latest_log.display());
        
        if let Ok(content) = fs::read_to_string(&latest_log) {
            let lines: Vec<&str> = content.lines().take(20).collect();
            eprintln!("First lines of the log:");
            for line in lines {
                eprintln!("  {}", line);
            }
        }
    } else {
        eprintln!("Log file NOT generated at {}", latest_log.display());
        
        let logs_dir = instance_dir.join("logs");
        if logs_dir.exists() {
            eprintln!("Contents of logs directory:");
            if let Ok(entries) = fs::read_dir(&logs_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    eprintln!("  - {} ({} bytes)", entry.file_name().to_string_lossy(), size);
                }
            }
        } else {
            eprintln!("  logs directory DOES NOT EXIST");
        }
    }
    
    if !status.success() && !was_stop_requested {
        // stdout/stderr were never piped for this child (they inherit the
        // launcher's), so there's no captured output to include here; check
        // latest.log (printed above) for details.
        return Err(anyhow!("Game process failed with status: {}", status));
    }

    Ok(())
}

// -----------------------------------------------
// FULL INSTALLATION (with cache warm-up)
// -----------------------------------------------

#[tauri::command]
async fn install_game(
    app: AppHandle,
    progress_channel: Channel<InstallProgress>,
    ram: u64,
    _java_args: String,
    java_path: String,
) -> Result<(), String> {
    eprintln!("=== STARTING FULL INSTALLATION ===");
    eprintln!("RAM: {}MB | Java: {}", ram, java_path);
    
    if ram < 512 {
        return Err("RAM minima: 512MB".to_string());
    }
    if java_path.trim().is_empty() {
        return Err("Java path cannot be empty".to_string());
    }
    if !Path::new(&java_path).exists() {
        return Err(format!("Java path no existe: {}", java_path));
    }

    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let shared_dir = get_shared_dir(&app_dir);
    let instances_dir = get_instances_dir(&app_dir);
    let instance_dir = get_instance_dir(&instances_dir);
    let instance_path = get_instance_json_path(&instance_dir);

    create_directories(&shared_dir, &instances_dir, &instance_dir).map_err(|e| e.to_string())?;

    let mut instance: InstanceJson = if instance_path.exists() {
        let data = fs::read_to_string(&instance_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())?
    } else {
        InstanceJson::default()
    };

    let version_json_path = shared_dir.join("versions").join(MINECRAFT_VERSION).join(format!("{}.json", MINECRAFT_VERSION));
    let forge_installer_path = shared_dir.join("cache").join("installers").join("forge-installer.jar");
    let client_jar_path = shared_dir.join("versions").join(MINECRAFT_VERSION).join(format!("{}.jar", MINECRAFT_VERSION));
    let forge_dir = shared_dir.join("versions").join(format!("{}-forge-{}", MINECRAFT_VERSION, FORGE_VERSION));
    let forge_version_json = forge_dir.join("version.json");

    eprintln!("=== Installation Check ===");
    eprintln!("Client jar exists: {}", client_jar_path.exists());
    eprintln!("Forge version.json exists: {}", forge_version_json.exists());
    let assets_ok = assets_seem_complete(&shared_dir, &version_json_path);
    eprintln!("Assets complete: {}", assets_ok);
    eprintln!("Instance installed flag: {}", instance.installed);
    eprintln!("=========================");

    if version_json_path.exists() && !is_json_valid(&version_json_path) {
        eprintln!("Removing corrupt version.json: {}", version_json_path.display());
        fs::remove_file(&version_json_path).map_err(|e| e.to_string())?;
    }
    if forge_installer_path.exists() && !is_file_valid(&forge_installer_path) {
        eprintln!("Removing corrupt forge-installer.jar: {}", forge_installer_path.display());
        fs::remove_file(&forge_installer_path).map_err(|e| e.to_string())?;
    }
    if forge_version_json.exists() && !is_json_valid(&forge_version_json) {
        eprintln!("Removing corrupt Forge version.json: {}", forge_version_json.display());
        fs::remove_file(&forge_version_json).map_err(|e| e.to_string())?;
    }
    if client_jar_path.exists() && !is_file_valid(&client_jar_path) {
        eprintln!("Removing corrupt client.jar: {}", client_jar_path.display());
        fs::remove_file(&client_jar_path).map_err(|e| e.to_string())?;
    }

    let assets_ok_now = assets_seem_complete(&shared_dir, &version_json_path);
    if !assets_ok_now {
        eprintln!("Assets incomplete or missing — forcing reinstallation.");
        instance.installed = false;
        fs::write(&instance_path, serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }

    let installed_ok = instance.installed
        && is_file_valid(&client_jar_path)
        && is_json_valid(&forge_version_json)
        && assets_ok_now;

    if installed_ok {
        eprintln!("Installation already complete. No need to reinstall.");
        return Ok(());
    }

    if instance.installed {
        eprintln!("instance.json says installed, but required files are missing or corrupt — reinstalling");
        instance.installed = false;
        fs::write(&instance_path, serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }

    let client = reqwest::Client::new();

    send_progress(&progress_channel, "Downloading manifest", 0, 1, 0.0, "Fetching version manifest...").await;
    let manifest = download_version_manifest(&client).await.map_err(|e| e.to_string())?;
    let version_url = manifest
        .versions
        .iter()
        .find(|v| v.id == MINECRAFT_VERSION)
        .ok_or_else(|| anyhow!("Version {} not found", MINECRAFT_VERSION))
        .map_err(|e| e.to_string())?
        .url
        .clone();

    if !is_json_valid(&version_json_path) {
        send_progress(&progress_channel, "Downloading version metadata", 0, 1, 0.0, "Downloading 1.20.1.json...").await;
        download_file(&client, &version_url, &version_json_path, None, Some(&progress_channel), "Downloading version metadata", "1.20.1.json").await.map_err(|e| e.to_string())?;
        if !is_json_valid(&version_json_path) {
            fs::remove_file(&version_json_path).map_err(|e| e.to_string())?;
            download_file(&client, &version_url, &version_json_path, None, Some(&progress_channel), "Downloading version metadata", "1.20.1.json").await.map_err(|e| e.to_string())?;
            if !is_json_valid(&version_json_path) {
                return Err("Failed to download valid version.json".to_string());
            }
        }
    }

    let mojang_json: Value = serde_json::from_reader(File::open(&version_json_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let downloads_client = mojang_json["downloads"]["client"].as_object()
        .ok_or_else(|| "Missing downloads.client in Mojang version.json")?;
    let client_url = downloads_client.get("url").and_then(|v| v.as_str())
        .ok_or_else(|| "Missing url in downloads.client")?;
    let client_sha1 = downloads_client.get("sha1").and_then(|v| v.as_str())
        .ok_or_else(|| "Missing sha1 in downloads.client")?;

    let java_exe = get_java_executable(&shared_dir, Some(&java_path))
        .await
        .map_err(|e| format!("Java 25 not found: {}", e))?;

    if !is_file_valid(&client_jar_path) {
        send_progress(&progress_channel, "Downloading Minecraft client", 0, 1, 0.0, "Downloading client.jar...").await;
        download_file(&client, client_url, &client_jar_path, Some(client_sha1), Some(&progress_channel), "Downloading Minecraft client", "client.jar").await.map_err(|e| e.to_string())?;
        if !is_file_valid(&client_jar_path) {
            return Err("Downloaded client.jar is empty or corrupt".to_string());
        }
    }

    let _log4j_path = download_log4j_config(
        &client,
        &mojang_json,
        &shared_dir,
        &progress_channel
    ).await.map_err(|e| e.to_string())?;
    eprintln!("Log4j config ready");

    if !is_file_valid(&forge_installer_path) {
        let forge_url = format! {
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{}-{}/forge-{}-{}-installer.jar",
            MINECRAFT_VERSION, FORGE_VERSION, MINECRAFT_VERSION, FORGE_VERSION
        };
        send_progress(&progress_channel, "Downloading Forge installer", 0, 1, 0.0, "Downloading forge installer...").await;
        download_file(&client, &forge_url, &forge_installer_path, None, Some(&progress_channel), "Downloading Forge installer", "forge-installer.jar").await.map_err(|e| e.to_string())?;
        if !is_file_valid(&forge_installer_path) {
            fs::remove_file(&forge_installer_path).map_err(|e| e.to_string())?;
            download_file(&client, &forge_url, &forge_installer_path, None, Some(&progress_channel), "Downloading Forge installer", "forge-installer.jar").await.map_err(|e| e.to_string())?;
            if !is_file_valid(&forge_installer_path) {
                return Err("Downloaded forge-installer.jar is empty or corrupt".to_string());
            }
        }
    }

    fs::create_dir_all(&forge_dir).map_err(|e| e.to_string())?;
    if !is_json_valid(&forge_version_json) {
        process_forge_installation(&shared_dir, &forge_installer_path, &forge_dir, &java_exe, &progress_channel).await.map_err(|e| e.to_string())?;
    } else {
        eprintln!("Forge version.json already valid — skipping reprocessing.");
    }

    let mojang_libs: Vec<Library> = if let Some(libs) = mojang_json["libraries"].as_array() {
        libs.iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .filter(library_applies)
            .collect()
    } else {
        Vec::new()
    };

    let mut library_list = mojang_libs;

    if is_json_valid(&forge_version_json) {
        let forge_json: Value = serde_json::from_reader(File::open(&forge_version_json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if let Some(libs) = forge_json["libraries"].as_array() {
            let forge_libs: Vec<Library> = libs.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .filter(library_applies)
                .collect();
            library_list.extend(forge_libs);
        }
        library_list = deduplicate_libraries(library_list);
    }

    send_progress(&progress_channel, "Downloading libraries", 0, library_list.len() as u64, 0.0, "Preparing...").await;
    download_libraries(&client, &shared_dir, &library_list, &progress_channel).await.map_err(|e| e.to_string())?;

    let asset_index_id = mojang_json["assetIndex"]["id"].as_str()
        .ok_or_else(|| "Missing assetIndex.id in Mojang version.json")?;
    let asset_index_sha1 = mojang_json["assetIndex"]["sha1"].as_str()
        .ok_or_else(|| "Missing assetIndex.sha1 in Mojang version.json")?;
    let asset_index_url = mojang_json["assetIndex"]["url"].as_str()
        .ok_or_else(|| "Missing assetIndex.url in Mojang version.json")?;
    let asset_index_size = mojang_json["assetIndex"]["size"].as_u64().unwrap_or(0);
    let asset_index_total_size = mojang_json["assetIndex"]["totalSize"].as_u64().unwrap_or(0);

    let asset_index = AssetIndex {
        id: asset_index_id.to_string(),
        sha1: asset_index_sha1.to_string(),
        size: asset_index_size,
        total_size: asset_index_total_size,
        url: asset_index_url.to_string(),
    };

    let assets_dir = shared_dir.join("assets");
    download_assets(&client, &asset_index, &assets_dir, &progress_channel).await.map_err(|e| e.to_string())?;

    let natives_dir = instance_dir.join("natives");
    extract_natives(&shared_dir, &library_list, &natives_dir).await.map_err(|e| e.to_string())?;

    instance.installed = true;
    fs::write(&instance_path, serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    eprintln!("=== INSTALLATION COMPLETE ===");
    eprintln!("Warming up mod cache in the background...");

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = warm_mod_cache(&app_handle).await {
            eprintln!("Error warming up mod cache: {}", e);
        } else {
            eprintln!("Mod cache warmed up successfully.");
        }
    });

    Ok(())
}

// -----------------------------------------------
// MAIN play() COMMAND
// -----------------------------------------------

#[tauri::command]
async fn play(
    app: AppHandle,
    _window: Window,
    progress_channel: Channel<InstallProgress>,
    ram: u64,
    java_args: String,
    java_path: String,
    account: Option<Account>,
) -> Result<(), String> {
    eprintln!("=== STARTING LAUNCH ===");
    eprintln!("RAM: {}MB | Java: {} | Account: {:?}", 
        ram, 
        java_path, 
        account.as_ref().map(|a| &a.username)
    );
    
    if ram < 512 {
        return Err("RAM minima: 512MB".to_string());
    }
    if java_path.trim().is_empty() {
        return Err("Java path cannot be empty".to_string());
    }
    if !Path::new(&java_path).exists() {
        return Err(format!("Java path no existe: {}", java_path));
    }

    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let shared_dir = get_shared_dir(&app_dir);
    let instances_dir = get_instances_dir(&app_dir);
    let instance_dir = get_instance_dir(&instances_dir);
    let instance_path = get_instance_json_path(&instance_dir);

    if !instance_path.exists() {
        return Err("The game isn't installed. Click 'Install' first.".to_string());
    }

    let instance: InstanceJson = {
        let data = fs::read_to_string(&instance_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())?
    };

    if !instance.installed {
        return Err("The game isn't installed. Click 'Install' first.".to_string());
    }

    let client_jar = shared_dir.join("versions").join(MINECRAFT_VERSION).join(format!("{}.jar", MINECRAFT_VERSION));
    let forge_version_json = shared_dir
        .join("versions")
        .join(format!("{}-forge-{}", MINECRAFT_VERSION, FORGE_VERSION))
        .join("version.json");

    if !is_file_valid(&client_jar) {
        return Err("client.jar not found. Reinstall the game.".to_string());
    }
    if !is_json_valid(&forge_version_json) {
        return Err("Forge isn't installed correctly. Reinstall the game.".to_string());
    }
    if !assets_seem_complete(&shared_dir, &shared_dir.join("versions").join(MINECRAFT_VERSION).join(format!("{}.json", MINECRAFT_VERSION))) {
        return Err("Assets incomplete. Reinstall the game.".to_string());
    }

    eprintln!("Installation verified. Launching game...");

    let mut instance = instance;
    instance.play_count += 1;
    instance.last_played = Some(chrono::Utc::now().to_rfc3339());
    fs::write(&instance_path, serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    launch_game_impl(
        &app,
        &progress_channel,
        &shared_dir,
        &instance_dir,
        account,
        ram,
        &java_args,
        &java_path,
    ).await.map_err(|e| e.to_string())?;

    Ok(())
}

// -----------------------------------------------
// stop_game
// -----------------------------------------------
// Stops the Minecraft process that was launched from the launcher, using the
// PID stored in GameProcessState. launch_game_impl detects that the exit
// was caused by this (stop_requested) and doesn't report it as an error.
#[tauri::command]
async fn stop_game(app: AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<GameProcessState>()
        .ok_or_else(|| "Game state not available".to_string())?;

    let pid = *state.pid.lock().unwrap();
    let Some(pid) = pid else {
        return Err("There's no game currently running.".to_string());
    };

    *state.stop_requested.lock().unwrap() = true;

    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .spawn()
            .map_err(|e| format!("Could not stop the game: {e}"))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("kill")
            .args(["-9", &pid.to_string()])
            .spawn()
            .map_err(|e| format!("Could not stop the game: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
async fn check_installation(app: AppHandle) -> Result<bool, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let instance_path = get_instance_json_path(&instance_dir);
    if !instance_path.exists() {
        return Ok(false);
    }
    let data = fs::read_to_string(&instance_path).map_err(|e| e.to_string())?;
    let instance: InstanceJson = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if !instance.installed {
        return Ok(false);
    }
    let client_jar = app_dir.join("Shared").join("versions").join(MINECRAFT_VERSION).join(format!("{}.jar", MINECRAFT_VERSION));
    let forge_version_json = app_dir
        .join("Shared")
        .join("versions")
        .join(format!("{}-forge-{}", MINECRAFT_VERSION, FORGE_VERSION))
        .join("version.json");
    Ok(is_file_valid(&client_jar) && is_json_valid(&forge_version_json))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStats {
    pub play_count: u64,
    pub last_played: Option<String>,
    pub total_time_played_secs: u64,
}

#[tauri::command]
async fn get_instance_stats(app: AppHandle) -> Result<InstanceStats, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let instance_path = get_instance_json_path(&instance_dir);

    if !instance_path.exists() {
        return Ok(InstanceStats {
            play_count: 0,
            last_played: None,
            total_time_played_secs: 0,
        });
    }

    let data = fs::read_to_string(&instance_path).map_err(|e| e.to_string())?;
    let instance: InstanceJson = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(InstanceStats {
        play_count: instance.play_count,
        last_played: instance.last_played,
        total_time_played_secs: instance.total_time_played_secs,
    })
}

// Debug helper: resets the accumulated playtime (and, optionally, the play
// count) without touching anything else in instance.json — no need to go
// find and hand-edit the file on disk.
#[tauri::command]
async fn reset_instance_playtime(app: AppHandle, reset_play_count: bool) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let instance_path = get_instance_json_path(&instance_dir);

    if !instance_path.exists() {
        return Ok(());
    }

    let data = fs::read_to_string(&instance_path).map_err(|e| e.to_string())?;
    let mut instance: InstanceJson = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    instance.total_time_played_secs = 0;
    if reset_play_count {
        instance.play_count = 0;
        instance.last_played = None;
    }
    fs::write(&instance_path, serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// -----------------------------------------------
// MOD FUNCTIONS (optimized)
// -----------------------------------------------

fn read_mod_info_optimized(path: &Path) -> Result<ModInfo> {
    let file = File::open(path)
        .with_context(|| format!("Could not open '{}'", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("'{}' is not a valid jar", path.display()))?;

    let mut toml_text = None;
    if let Ok(mut f) = archive.by_name("META-INF/mods.toml") {
        let mut content = String::new();
        if f.read_to_string(&mut content).is_ok() {
            toml_text = Some(content);
        }
    }

    let raw_file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let enabled = !raw_file_name.to_lowercase().ends_with(".disabled");
    let file_name = if enabled {
        raw_file_name.clone()
    } else {
        raw_file_name
            .strip_suffix(".disabled")
            .or_else(|| raw_file_name.strip_suffix(".DISABLED"))
            .unwrap_or(&raw_file_name)
            .to_string()
    };
    let default_name = file_name.trim_end_matches(".jar").to_string();

    let (mod_id, name, version_raw, description) = match &toml_text {
        Some(text) => {
            let parsed: ModsToml = toml::from_str(text).unwrap_or_default();
            let entry = parsed.mods.into_iter().next().unwrap_or_default();
            let name = if entry.display_name.trim().is_empty() {
                default_name.clone()
            } else {
                entry.display_name
            };
            (entry.mod_id, name, entry.version, entry.description)
        }
        None => (String::new(), default_name, String::new(), None),
    };

    let version = if version_raw.trim().is_empty() || version_raw.contains("${") {
        if let Ok(mut f) = archive.by_name("META-INF/MANIFEST.MF") {
            let mut content = String::new();
            if f.read_to_string(&mut content).is_ok() {
                for line in content.lines() {
                    if let Some(rest) = line.strip_prefix("Implementation-Version: ") {
                        let v = rest.trim();
                        if !v.is_empty() {
                            return Ok(ModInfo {
                                file_name,
                                mod_id,
                                name,
                                version: v.to_string(),
                                description,
                                fingerprint: 0,
                                enabled,
                                icon_url: None,
                            });
                        }
                    }
                }
            }
        }
        "unknown".to_string()
    } else {
        version_raw
    };

    Ok(ModInfo {
        file_name,
        mod_id,
        name,
        version,
        description,
        fingerprint: 0,
        enabled,
        icon_url: None,
    })
}

struct Murmur2Hasher {
    h: u32,
    pending: [u8; 4],
    pending_len: usize,
}

impl Murmur2Hasher {
    fn new(seed: u32, len: u32) -> Self {
        Self {
            h: seed ^ len,
            pending: [0; 4],
            pending_len: 0,
        }
    }

    fn update(&mut self, byte: u8) {
        self.pending[self.pending_len] = byte;
        self.pending_len += 1;
        if self.pending_len == 4 {
            self.process_block(u32::from_le_bytes(self.pending));
            self.pending_len = 0;
        }
    }

    fn process_block(&mut self, k: u32) {
        const M: u32 = 0x5bd1e995;
        const R: u32 = 24;
        let mut k = k;
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        self.h = self.h.wrapping_mul(M);
        self.h ^= k;
    }

    fn finalize(mut self) -> u32 {
        let mut k = 0u32;
        if self.pending_len > 0 {
            for i in 0..self.pending_len {
                k |= (self.pending[i] as u32) << (8 * i);
            }
            self.h ^= k;
            self.h = self.h.wrapping_mul(0x5bd1e995);
        }
        self.h ^= self.h >> 13;
        self.h = self.h.wrapping_mul(0x5bd1e995);
        self.h ^= self.h >> 15;
        self.h
    }
}

fn is_curseforge_ignored_byte(byte: u8) -> bool {
    matches!(byte, 9 | 10 | 13 | 32)
}

fn count_curseforge_fingerprint_bytes(path: &Path) -> Result<u32> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0; 8192];
    let mut len: u32 = 0;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            if !is_curseforge_ignored_byte(b) {
                len = len.wrapping_add(1);
            }
        }
    }
    Ok(len)
}

fn curseforge_fingerprint_streaming(path: &Path) -> Result<u32> {
    let len = count_curseforge_fingerprint_bytes(path)?;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0; 8192];
    let mut hasher = Murmur2Hasher::new(1, len);
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        for &b in &buffer[..n] {
            if !is_curseforge_ignored_byte(b) {
                hasher.update(b);
            }
        }
    }
    Ok(hasher.finalize())
}

fn mod_info_cache_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_dir = get_app_dir(app)?;
    Ok(app_dir.join("mod_info_cache.json"))
}

fn load_mod_info_cache(path: &Path) -> HashMap<String, ModCacheEntry> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_mod_info_cache_atomic(path: &Path, cache: &HashMap<String, ModCacheEntry>) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    let bytes = serde_json::to_vec(cache)?;
    fs::write(&temp_path, &bytes)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

fn file_fingerprint_meta(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((size, modified))
}

fn cf_cache_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let app_dir = get_app_dir(app)?;
    Ok(app_dir.join("curseforge_cache.json"))
}

fn load_cf_cache(path: &Path) -> HashMap<u32, Option<CfModMeta>> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_cf_cache_atomic(path: &Path, cache: &HashMap<u32, Option<CfModMeta>>) -> Result<()> {
    let temp_path = path.with_extension("tmp");
    let bytes = serde_json::to_vec(cache)?;
    fs::write(&temp_path, &bytes)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfModMeta {
    pub fingerprint: u32,
    pub mod_id: u32,
    pub name: String,
    pub icon_url: Option<String>,
    pub website_url: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfFingerprintResponse {
    data: CfFingerprintData,
}

#[derive(Debug, Deserialize)]
struct CfFingerprintData {
    #[serde(rename = "exactMatches", default)]
    exact_matches: Vec<CfExactMatch>,
}

#[derive(Debug, Deserialize)]
struct CfExactMatch {
    file: CfFile,
}

#[derive(Debug, Deserialize)]
struct CfFile {
    #[serde(rename = "modId")]
    mod_id: u32,
    #[serde(rename = "fileFingerprint", default)]
    file_fingerprint: u32,
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
    #[serde(rename = "fileName", default)]
    file_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfModsResponse {
    data: Vec<CfModData>,
}

#[derive(Debug, Deserialize)]
struct CfModData {
    id: u32,
    name: String,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    links: Option<CfLinks>,
}

#[derive(Debug, Deserialize)]
struct CfLogo {
    #[serde(rename = "thumbnailUrl", default)]
    thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CfLinks {
    #[serde(rename = "websiteUrl", default)]
    website_url: Option<String>,
}

async fn get_cf_metadata(
    app: &AppHandle,
    fingerprints: &[u32],
) -> Result<HashMap<u32, CfModMeta>> {
    if fingerprints.is_empty() {
        return Ok(HashMap::new());
    }

    let cache_path = cf_cache_path(app)?;
    let mut cache = load_cf_cache(&cache_path);
    let mut result = HashMap::new();
    let mut missing: Vec<u32> = Vec::new();

    for fp in fingerprints {
        if let Some(Some(meta)) = cache.get(fp) {
            result.insert(*fp, meta.clone());
        } else if !cache.contains_key(fp) {
            missing.push(*fp);
        }
    }

    if missing.is_empty() {
        return Ok(result);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| Client::new());
    let api_key = CURSEFORGE_API_KEY;

    let fp_resp = client
        .post("https://api.curseforge.com/v1/fingerprints")
        .header("x-api-key", api_key)
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "fingerprints": missing }))
        .send()
        .await
        .map_err(|e| anyhow!("Could not reach the CurseForge API: {e}"))?
        .json::<CfFingerprintResponse>()
        .await
        .map_err(|e| anyhow!("Respuesta inesperada de CurseForge (fingerprints): {e}"))?;

    let mut fp_to_file: HashMap<u32, &CfFile> = HashMap::new();
    for m in &fp_resp.data.exact_matches {
        if m.file.file_fingerprint != 0 {
            fp_to_file.insert(m.file.file_fingerprint, &m.file);
        }
    }

    let mod_ids: Vec<u32> = fp_to_file
        .values()
        .map(|f| f.mod_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut mod_data_by_id: HashMap<u32, CfModData> = HashMap::new();
    if !mod_ids.is_empty() {
        let mods_resp = client
            .post("https://api.curseforge.com/v1/mods")
            .header("x-api-key", api_key)
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "modIds": mod_ids }))
            .send()
            .await
            .map_err(|e| anyhow!("Could not reach the CurseForge API: {e}"))?
            .json::<CfModsResponse>()
            .await
            .map_err(|e| anyhow!("Respuesta inesperada de CurseForge (mods): {e}"))?;

        for m in mods_resp.data {
            mod_data_by_id.insert(m.id, m);
        }
    }

    for fp in &missing {
        if let Some(file) = fp_to_file.get(fp) {
            if let Some(m) = mod_data_by_id.get(&file.mod_id) {
                let version = file.display_name.clone().or(file.file_name.clone());
                let meta = CfModMeta {
                    fingerprint: *fp,
                    mod_id: m.id,
                    name: m.name.clone(),
                    icon_url: m.logo.as_ref().and_then(|l| l.thumbnail_url.clone()),
                    website_url: m.links.as_ref().and_then(|l| l.website_url.clone()),
                    version,
                };
                cache.insert(*fp, Some(meta.clone()));
                result.insert(*fp, meta);
            } else {
                cache.insert(*fp, None);
            }
        } else {
            cache.insert(*fp, None);
        }
    }

    let _ = save_cf_cache_atomic(&cache_path, &cache);

    Ok(result)
}

// -----------------------------------------------
// COMANDO: list_installed_mods
// -----------------------------------------------

#[tauri::command]
async fn list_installed_mods(
    app: AppHandle,
    progress: Channel<Vec<ModInfo>>,
) -> Result<Vec<ModInfo>, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        let cache_path = mod_info_cache_path(&app).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&cache_path);
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&mods_dir).map_err(|e| e.to_string())?;
    let jar_paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_lowercase(),
                None => return false,
            };
            name.ends_with(".jar") || name.ends_with(".jar.disabled")
        })
        .collect();

    if jar_paths.is_empty() {
        let cache_path = mod_info_cache_path(&app).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&cache_path);
        return Ok(Vec::new());
    }

    let cache_path = mod_info_cache_path(&app).map_err(|e| e.to_string())?;
    let old_cache = load_mod_info_cache(&cache_path);

    let mut first_batch = Vec::with_capacity(BATCH_SIZE);
    let mut to_parse: Vec<PathBuf> = Vec::new();
    let mut fresh_cache: HashMap<String, ModCacheEntry> = HashMap::with_capacity(jar_paths.len());
    let mut dirty = false;
    let mut all_mods_final: Vec<ModInfo> = Vec::new();

    for path in &jar_paths {
        let key = path
            .strip_prefix(&mods_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let meta = file_fingerprint_meta(path);

        let cached_entry = match (&meta, old_cache.get(&key)) {
            (Some((size, modified)), Some(entry)) if entry.size == *size && entry.modified == *modified => {
                let mut info = entry.info.clone();
                info.enabled = !path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase().ends_with(".disabled"))
                    .unwrap_or(false);
                fresh_cache.insert(key.clone(), ModCacheEntry {
                    size: *size,
                    modified: *modified,
                    info: info.clone(),
                });
                Some(info)
            }
            _ => None,
        };

        if let Some(info) = cached_entry {
            all_mods_final.push(info.clone());
            first_batch.push(info);
            if first_batch.len() >= BATCH_SIZE {
                let _ = progress.send(first_batch);
                first_batch = Vec::with_capacity(BATCH_SIZE);
            }
        } else {
            let raw_file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let enabled = !raw_file_name.to_lowercase().ends_with(".disabled");
            let file_name = if enabled {
                raw_file_name.clone()
            } else {
                raw_file_name
                    .strip_suffix(".disabled")
                    .unwrap_or(&raw_file_name)
                    .to_string()
            };
            let name = file_name.trim_end_matches(".jar").to_string();
            let minimal = ModInfo {
                file_name,
                mod_id: String::new(),
                name,
                version: "unknown".to_string(),
                description: None,
                fingerprint: 0,
                enabled,
                icon_url: None,
            };
            all_mods_final.push(minimal.clone());
            first_batch.push(minimal);
            if first_batch.len() >= BATCH_SIZE {
                let _ = progress.send(first_batch);
                first_batch = Vec::with_capacity(BATCH_SIZE);
            }
            to_parse.push(path.clone());
        }
    }

    if !first_batch.is_empty() {
        let _ = progress.send(first_batch);
    }

    if !to_parse.is_empty() {
        let concurrency = 2;
        let sem = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(to_parse.len());
        for path in to_parse {
            let permit = sem.clone().acquire_owned().await.map_err(|e| e.to_string())?;
            let mods_dir_clone = mods_dir.clone();
            handles.push(tauri::async_runtime::spawn_blocking(move || {
                let _permit = permit;
                let key = path
                    .strip_prefix(&mods_dir_clone)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let (size, modified) = file_fingerprint_meta(&path)
                    .unwrap_or((0, 0));
                let info = match read_mod_info_optimized(&path) {
                    Ok(mut info) => {
                        info.fingerprint = 0;
                        info
                    }
                    Err(e) => {
                        eprintln!("Could not read info for mod '{}': {}", path.display(), e);
                        let raw_file_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let enabled = !raw_file_name.to_lowercase().ends_with(".disabled");
                        let file_name = if enabled {
                            raw_file_name.clone()
                        } else {
                            raw_file_name
                                .strip_suffix(".disabled")
                                .unwrap_or(&raw_file_name)
                                .to_string()
                        };
                        let name = file_name.trim_end_matches(".jar").to_string();
                        ModInfo {
                            file_name,
                            mod_id: String::new(),
                            name,
                            version: "unknown".to_string(),
                            description: None,
                            fingerprint: 0,
                            enabled,
                            icon_url: None,
                        }
                    }
                };
                (key, size, modified, info)
            }));
        }

        let mut updates = Vec::new();
        for handle in handles {
            if let Ok((key, size, modified, info)) = handle.await {
                if let Some(pos) = all_mods_final.iter().position(|m| m.file_name == info.file_name) {
                    all_mods_final[pos] = info.clone();
                } else {
                    all_mods_final.push(info.clone());
                }
                fresh_cache.insert(key, ModCacheEntry { size, modified, info: info.clone() });
                updates.push(info);
                dirty = true;
                if updates.len() >= BATCH_SIZE {
                    let _ = progress.send(updates);
                    updates = Vec::new();
                }
            }
        }
        if !updates.is_empty() {
            let _ = progress.send(updates);
        }
    }

    let mut pending_icon_fingerprints: Vec<u32> = Vec::with_capacity(ICON_BATCH_SIZE);

    for path in &jar_paths {
        let raw_file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let enabled = !raw_file_name.to_lowercase().ends_with(".disabled");
        let file_name = if enabled {
            raw_file_name.clone()
        } else {
            raw_file_name
                .strip_suffix(".disabled")
                .or_else(|| raw_file_name.strip_suffix(".DISABLED"))
                .unwrap_or(&raw_file_name)
                .to_string()
        };

        let Some(pos) = all_mods_final
            .iter()
            .position(|m| m.file_name == file_name && (m.icon_url.is_none() || m.version == "unknown"))
        else {
            continue;
        };

        let fingerprint = if all_mods_final[pos].fingerprint != 0 {
            all_mods_final[pos].fingerprint
        } else {
            let path_for_task = path.clone();
            match tauri::async_runtime::spawn_blocking(move || {
                curseforge_fingerprint_streaming(&path_for_task).ok()
            })
            .await
            {
                Ok(Some(fp)) if fp != 0 => fp,
                _ => continue,
            }
        };

        all_mods_final[pos].fingerprint = fingerprint;
        let key = path
            .strip_prefix(&mods_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let (size, modified) = file_fingerprint_meta(path).unwrap_or((0, 0));
        fresh_cache.insert(
            key,
            ModCacheEntry {
                size,
                modified,
                info: all_mods_final[pos].clone(),
            },
        );
        dirty = true;
        pending_icon_fingerprints.push(fingerprint);

        if pending_icon_fingerprints.len() >= ICON_BATCH_SIZE {
            let cf_future = get_cf_metadata(&app, &pending_icon_fingerprints);
            if let Ok(Ok(cf_map)) = tokio::time::timeout(Duration::from_secs(10), cf_future).await {
                let mut updates = Vec::new();
                for mod_info in &mut all_mods_final {
                    if !pending_icon_fingerprints.contains(&mod_info.fingerprint) {
                        continue;
                    }
                    if let Some(meta) = cf_map.get(&mod_info.fingerprint) {
                        if let Some(icon_url) = &meta.icon_url {
                            mod_info.icon_url = Some(icon_url.clone());
                        }
                        if let Some(version) = &meta.version {
                            if mod_info.version == "unknown" || mod_info.version.is_empty() {
                                mod_info.version = version.clone();
                            }
                        }
                        updates.push(mod_info.clone());
                        if let Some(entry) = fresh_cache
                            .values_mut()
                            .find(|entry| entry.info.file_name == mod_info.file_name)
                        {
                            entry.info.icon_url = mod_info.icon_url.clone();
                            entry.info.version = mod_info.version.clone();
                            entry.info.fingerprint = mod_info.fingerprint;
                        }
                        dirty = true;
                    }
                }
                if !updates.is_empty() {
                    let _ = progress.send(updates);
                }
            } else {
                eprintln!("Timeout or error fetching CurseForge metadata. Continuing without it.");
            }
            pending_icon_fingerprints.clear();
        }
    }

    if !pending_icon_fingerprints.is_empty() {
        let cf_future = get_cf_metadata(&app, &pending_icon_fingerprints);
        if let Ok(Ok(cf_map)) = tokio::time::timeout(Duration::from_secs(10), cf_future).await {
            let mut updates = Vec::new();
            for mod_info in &mut all_mods_final {
                if !pending_icon_fingerprints.contains(&mod_info.fingerprint) {
                    continue;
                }
                if let Some(meta) = cf_map.get(&mod_info.fingerprint) {
                    if let Some(icon_url) = &meta.icon_url {
                        mod_info.icon_url = Some(icon_url.clone());
                    }
                    if let Some(version) = &meta.version {
                        if mod_info.version == "unknown" || mod_info.version.is_empty() {
                            mod_info.version = version.clone();
                        }
                    }
                    updates.push(mod_info.clone());
                    if let Some(entry) = fresh_cache
                        .values_mut()
                        .find(|entry| entry.info.file_name == mod_info.file_name)
                    {
                        entry.info.icon_url = mod_info.icon_url.clone();
                        entry.info.version = mod_info.version.clone();
                        entry.info.fingerprint = mod_info.fingerprint;
                    }
                    dirty = true;
                }
            }
            if !updates.is_empty() {
                let _ = progress.send(updates);
            }
        } else {
            eprintln!("Timeout or error fetching CurseForge metadata. Continuing without it.");
        }
    }
    if dirty {
        if let Err(e) = save_mod_info_cache_atomic(&cache_path, &fresh_cache) {
            eprintln!("Error saving mod cache: {}", e);
        }
    }

    Ok(all_mods_final)
}

// -----------------------------------------------
// set_mod_enabled (optimized)
// -----------------------------------------------

#[tauri::command]
async fn set_mod_enabled(app: AppHandle, file_name: String, enabled: bool) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let mods_dir = instance_dir.join("mods");

    let enabled_path = mods_dir.join(&file_name);
    let disabled_path = mods_dir.join(format!("{file_name}.disabled"));

    if enabled {
        if enabled_path.exists() {
            return Ok(());
        }
        if !disabled_path.exists() {
            return Err(format!("Could not find mod file '{file_name}'"));
        }
        fs::rename(&disabled_path, &enabled_path)
            .map_err(|e| format!("Could not enable '{file_name}': {e}"))?;
    } else {
        if disabled_path.exists() {
            return Ok(());
        }
        if !enabled_path.exists() {
            return Err(format!("Could not find mod file '{file_name}'"));
        }
        fs::rename(&enabled_path, &disabled_path)
            .map_err(|e| format!("Could not disable '{file_name}': {e}"))?;
    }

    let cache_path = mod_info_cache_path(&app).map_err(|e| e.to_string())?;
    let mut cache = load_mod_info_cache(&cache_path);
    for (_, entry) in cache.iter_mut() {
        if entry.info.file_name == file_name {
            entry.info.enabled = enabled;
            break;
        }
    }
    if let Err(e) = save_mod_info_cache_atomic(&cache_path, &cache) {
        eprintln!("Error updating mod cache: {}", e);
    }

    Ok(())
}

// -----------------------------------------------
// show_mod_in_folder
// -----------------------------------------------
// Opens the OS file explorer and, when the
// platform allows it, leaves the mod file already selected/highlighted.
// `file_name` always arrives WITHOUT the ".disabled" suffix (same as in
// set_mod_enabled), so here we resolve which of the two paths
// (enabled or disabled) actually exists on disk.
#[tauri::command]
async fn show_mod_in_folder(app: AppHandle, file_name: String) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let mods_dir = instance_dir.join("mods");

    let enabled_path = mods_dir.join(&file_name);
    let disabled_path = mods_dir.join(format!("{file_name}.disabled"));

    let target = if enabled_path.exists() {
        enabled_path
    } else if disabled_path.exists() {
        disabled_path
    } else {
        return Err(format!("Could not find mod file '{file_name}'"));
    };

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &target.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Could not open the file explorer: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &target.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Could not open Finder: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        // Most file managers on Linux don't have a standard
        // way to "select" a specific file, so we open
        // the folder that contains it.
        let folder = target.parent().unwrap_or(&mods_dir);
        Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Could not open the file manager: {e}"))?;
    }

    Ok(())
}

// -----------------------------------------------
// Generic content: shaderpacks / resourcepacks
// -----------------------------------------------
// Reuses the same enable/disable pattern (".disabled" suffix)
// already used by mods, but generically so we don't duplicate the same code
// 3 times. `kind` only accepts "shaderpacks" or "resourcepacks".

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItemInfo {
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub name: String,
    pub enabled: bool,
    // Remote icon obtained from CurseForge by fingerprint (same as mods).
    // Takes priority over `icon_data` when available.
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<String>,
    // Local icon extracted from the file itself (e.g. pack.png), used as
    // a fallback when the pack isn't found on CurseForge.
    #[serde(rename = "iconData")]
    pub icon_data: Option<String>,
}

fn content_kind_dir(instance_dir: &Path, kind: &str) -> Result<PathBuf, String> {
    match kind {
        "shaderpacks" => Ok(instance_dir.join("shaderpacks")),
        "resourcepacks" => Ok(instance_dir.join("resourcepacks")),
        _ => Err(format!("Invalid content type: {kind}")),
    }
}

fn strip_disabled_suffix(name: &str) -> (&str, bool) {
    match name.strip_suffix(".disabled") {
        Some(stripped) => (stripped, false),
        None => (name, true),
    }
}

fn display_name_for_pack(name: &str) -> String {
    // Strips the .zip extension to show a cleaner name in the UI.
    name.strip_suffix(".zip").unwrap_or(name).to_string()
}

// Safety limit to avoid loading huge images into memory/UI.
const MAX_PACK_ICON_BYTES: u64 = 2 * 1024 * 1024; // 2MB

fn png_bytes_to_data_url(bytes: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{b64}")
}

// Looks for an icon inside an already-extracted resourcepack/shaderpack
// folder. `pack.png` is Minecraft's standard for resource packs;
// there's no standard for shaderpacks, so we try common names
// on a best-effort basis.
fn read_folder_icon_as_data_url(dir: &Path, candidates: &[&str]) -> Option<String> {
    for candidate in candidates {
        let path = dir.join(candidate);
        if !path.is_file() {
            continue;
        }
        if let Ok(meta) = fs::metadata(&path) {
            if meta.len() == 0 || meta.len() > MAX_PACK_ICON_BYTES {
                continue;
            }
        }
        if let Ok(bytes) = fs::read(&path) {
            return Some(png_bytes_to_data_url(&bytes));
        }
    }
    None
}

// Same as `read_folder_icon_as_data_url` but for packs packaged as
// .zip. Only looks in the zip's root (no subfolders), which is where
// `pack.png` lives in a valid resourcepack.
fn read_zip_icon_as_data_url(path: &Path, candidates: &[&str]) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).ok()?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry.name().to_string();
        if entry_name.contains('/') || entry_name.contains('\\') {
            continue;
        }
        let lower = entry_name.to_lowercase();
        if !candidates.iter().any(|c| lower == *c) {
            continue;
        }
        if entry.size() == 0 || entry.size() > MAX_PACK_ICON_BYTES {
            return None;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut bytes).is_err() {
            return None;
        }
        return Some(png_bytes_to_data_url(&bytes));
    }
    None
}

fn extract_pack_icon(path: &Path, is_dir: bool, kind: &str) -> Option<String> {
    let candidates: &[&str] = match kind {
        "resourcepacks" => &["pack.png"],
        // Shaderpacks don't have a standard icon; we try a few
        // common names that some authors include.
        _ => &["icon.png", "pack.png", "preview.png"],
    };
    if is_dir {
        read_folder_icon_as_data_url(path, candidates)
    } else {
        read_zip_icon_as_data_url(path, candidates)
    }
}

#[tauri::command]
async fn list_content_items(app: AppHandle, kind: String) -> Result<Vec<ContentItemInfo>, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let dir = content_kind_dir(&instance_dir, &kind)?;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    let mut items: Vec<ContentItemInfo> = Vec::new();
    // Only packaged files (.zip) can be identified on
    // CurseForge by fingerprint; uncompressed folders keep
    // their local icon (if they have one) or the initials.
    let mut fingerprint_targets: Vec<(usize, PathBuf)> = Vec::new();

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let raw_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Ignores hidden/system files (e.g. .DS_Store).
        if raw_name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let (base_name, enabled) = strip_disabled_suffix(&raw_name);

        // We accept zips and folders (packs may come uncompressed),
        // but we ignore loose files that don't look like a real pack.
        if !is_dir && !base_name.to_lowercase().ends_with(".zip") {
            continue;
        }

        let icon_data = extract_pack_icon(&path, is_dir, &kind);

        let index = items.len();
        items.push(ContentItemInfo {
            file_name: base_name.to_string(),
            name: display_name_for_pack(base_name),
            enabled,
            icon_url: None,
            icon_data,
        });

        if !is_dir {
            fingerprint_targets.push((index, path.clone()));
        }
    }

    if !fingerprint_targets.is_empty() {
        let concurrency = 4;
        let sem = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(fingerprint_targets.len());
        for (index, path) in fingerprint_targets {
            let permit = sem.clone().acquire_owned().await.map_err(|e| e.to_string())?;
            handles.push(tauri::async_runtime::spawn_blocking(move || {
                let _permit = permit;
                let fp = curseforge_fingerprint_streaming(&path)
                    .ok()
                    .filter(|&fp| fp != 0);
                (index, fp)
            }));
        }

        let mut fp_by_index: HashMap<usize, u32> = HashMap::new();
        let mut all_fingerprints: Vec<u32> = Vec::new();
        for handle in handles {
            if let Ok((index, Some(fp))) = handle.await {
                fp_by_index.insert(index, fp);
                all_fingerprints.push(fp);
            }
        }

        if !all_fingerprints.is_empty() {
            let cf_future = get_cf_metadata(&app, &all_fingerprints);
            match tokio::time::timeout(Duration::from_secs(10), cf_future).await {
                Ok(Ok(cf_map)) => {
                    for (index, fp) in &fp_by_index {
                        if let Some(meta) = cf_map.get(fp) {
                            if meta.icon_url.is_some() {
                                items[*index].icon_url = meta.icon_url.clone();
                            }
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "Timeout o error obteniendo iconos de CurseForge para '{kind}'. Continuando sin ellos."
                    );
                }
            }
        }
    }

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(items)
}

#[tauri::command]
async fn set_content_item_enabled(
    app: AppHandle,
    kind: String,
    file_name: String,
    enabled: bool,
) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let dir = content_kind_dir(&instance_dir, &kind)?;

    let enabled_path = dir.join(&file_name);
    let disabled_path = dir.join(format!("{file_name}.disabled"));

    if enabled {
        if disabled_path.exists() {
            fs::rename(&disabled_path, &enabled_path).map_err(|e| e.to_string())?;
        } else if !enabled_path.exists() {
            return Err(format!("Could not find '{file_name}'"));
        }
    } else if enabled_path.exists() {
        fs::rename(&enabled_path, &disabled_path).map_err(|e| e.to_string())?;
    } else if !disabled_path.exists() {
        return Err(format!("Could not find '{file_name}'"));
    }

    Ok(())
}

// Opens the system's file explorer and, when the platform
// allows it, leaves the file/folder already selected.
// fallback_dir solo se usa dentro del bloque #[cfg(target_os = "linux")];
// en Windows/macOS el compilador lo ve "sin usar" porque ese bloque se
// descarta en compilación. Es un falso positivo de plataforma, no un bug.
#[allow(unused_variables)]
fn reveal_path_in_file_manager(target: &Path, fallback_dir: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &target.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Could not open the file explorer: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &target.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Could not open Finder: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        let folder = target.parent().unwrap_or(fallback_dir);
        Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Could not open the file manager: {e}"))?;
    }

    Ok(())
}

// Opens a folder directly (without selecting any particular file).
fn open_folder(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Could not open the file explorer: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Could not open Finder: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Could not open the file manager: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
async fn show_content_item_in_folder(app: AppHandle, kind: String, file_name: String) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let dir = content_kind_dir(&instance_dir, &kind)?;

    let enabled_path = dir.join(&file_name);
    let disabled_path = dir.join(format!("{file_name}.disabled"));

    let target = if enabled_path.exists() {
        enabled_path
    } else if disabled_path.exists() {
        disabled_path
    } else {
        return Err(format!("Could not find '{file_name}'"));
    };

    reveal_path_in_file_manager(&target, &dir)
}

// -----------------------------------------------
// Changelog (Modpack Update Checker compatible)
// -----------------------------------------------
// Reads the same GitHub-hosted `meta.json` + `versions/<id>/changelog.txt`
// files that the in-game "Modpack Update Checker" mod already points to, so
// the launcher and the mod always show the exact same changelog per version.

const CHANGELOG_BASE_URL: &str =
    "https://raw.githubusercontent.com/SaberY24/Beyond-Promised-Sparks/main";
const CHANGELOG_CACHE_TTL_SECS: u64 = 15 * 60; // 15 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogVersion {
    pub id: String,
    pub released_at: i64,
    pub content: String,
    pub downloads: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct ChangelogMeta {
    #[serde(default)]
    versions: Vec<ChangelogMetaEntry>,
}

#[derive(Debug, Deserialize)]
struct ChangelogMetaEntry {
    id: String,
    #[serde(rename = "releasedAt")]
    released_at: i64,
    #[serde(default)]
    promotions: Option<ChangelogPromotions>,
}

#[derive(Debug, Deserialize)]
struct ChangelogPromotions {
    #[serde(default)]
    downloads: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogCache {
    fetched_at: u64,
    versions: Vec<ChangelogVersion>,
}

fn changelog_cache_path(app: &AppHandle) -> Result<PathBuf> {
    let app_dir = get_app_dir(app)?;
    Ok(get_shared_dir(&app_dir).join("changelog_cache.json"))
}

fn read_changelog_cache(path: &Path) -> Option<ChangelogCache> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice::<ChangelogCache>(&bytes).ok()
}

fn write_changelog_cache(path: &Path, cache: &ChangelogCache) {
    if let Ok(json) = serde_json::to_vec_pretty(cache) {
        let _ = fs::write(path, json);
    }
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// -----------------------------------------------
// fetch_changelogs
// -----------------------------------------------
// Downloads meta.json plus every versions/<id>/changelog.txt from the
// modpack's GitHub repo (the same source Modpack Update Checker reads
// in-game) and returns them sorted newest-first. Results are cached on disk
// for CHANGELOG_CACHE_TTL_SECS to avoid hitting GitHub on every Home tab
// visit; pass `force_refresh: true` to bypass the cache.
#[tauri::command]
async fn fetch_changelogs(app: AppHandle, force_refresh: bool) -> Result<Vec<ChangelogVersion>, String> {
    let cache_path = changelog_cache_path(&app).map_err(|e| e.to_string())?;

    if !force_refresh {
        if let Some(cache) = read_changelog_cache(&cache_path) {
            if unix_now_secs().saturating_sub(cache.fetched_at) < CHANGELOG_CACHE_TTL_SECS {
                return Ok(cache.versions);
            }
        }
    }

    let client = Client::new();

    let meta: ChangelogMeta = client
        .get(format!("{CHANGELOG_BASE_URL}/meta.json"))
        .send()
        .await
        .map_err(|e| format!("Could not reach the changelog repository: {e}"))?
        .json()
        .await
        .map_err(|e| format!("meta.json has an unexpected format: {e}"))?;

    let mut versions = Vec::with_capacity(meta.versions.len());
    for entry in meta.versions {
        let url = format!("{CHANGELOG_BASE_URL}/versions/{}/changelog.txt", entry.id);
        let content = match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => resp.text().await.unwrap_or_default(),
            _ => String::new(),
        };

        versions.push(ChangelogVersion {
            id: entry.id,
            released_at: entry.released_at,
            content,
            downloads: entry.promotions.and_then(|p| p.downloads),
        });
    }

    // Newest version first.
    versions.sort_by(|a, b| b.released_at.cmp(&a.released_at));

    write_changelog_cache(
        &cache_path,
        &ChangelogCache {
            fetched_at: unix_now_secs(),
            versions: versions.clone(),
        },
    );

    Ok(versions)
}

// -----------------------------------------------
// MODPACK: sync via GitHub Releases
// -----------------------------------------------

#[derive(Debug, Deserialize, Clone)]
struct GhReleaseAsset {
    id: u64,
    name: String,
    size: u64,
    updated_at: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize, Clone)]
struct GhRelease {
    tag_name: String,
    published_at: String,
    assets: Vec<GhReleaseAsset>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct ModpackState {
    tag_name: String,
    asset_id: u64,
    updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInfo {
    pub version: String,
    pub published_at: String,
    pub size: u64,
    pub up_to_date: bool,
}

async fn fetch_latest_modpack_release(client: &Client) -> Result<GhRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        MODPACK_REPO_OWNER, MODPACK_REPO_NAME
    );
    let resp = client
        .get(&url)
        // GitHub's API rejects requests without a User-Agent.
        .header("User-Agent", "BeyondPromisedSparksLauncher")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("Could not reach the GitHub API")?;

    if !resp.status().is_success() {
        return Err(anyhow!("GitHub API respondio con estado {}", resp.status()));
    }

    resp.json::<GhRelease>()
        .await
        .context("Respuesta inesperada de la API de GitHub")
}

fn find_modpack_asset(release: &GhRelease) -> Option<&GhReleaseAsset> {
    release.assets.iter().find(|a| a.name == MODPACK_ASSET_NAME)
}

fn modpack_state_path(instance_dir: &Path) -> PathBuf {
    instance_dir.join("modpack_state.json")
}

fn read_modpack_state(instance_dir: &Path) -> ModpackState {
    fs::read_to_string(modpack_state_path(instance_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_modpack_state(instance_dir: &Path, state: &ModpackState) -> Result<()> {
    fs::write(modpack_state_path(instance_dir), serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn collect_relative_files(root: &Path) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    fn walk(base: &Path, dir: &Path, out: &mut HashSet<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                out.insert(rel.to_path_buf());
            }
        }
    }
    if root.exists() {
        walk(root, root, &mut out);
    }
    out
}

fn remove_empty_dirs(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_empty_dirs(&path);
            if fs::read_dir(&path).map(|mut d| d.next().is_none()).unwrap_or(false) {
                let _ = fs::remove_dir(&path);
            }
        }
    }
}

// Deletes any file in `dir` that no longer exists in `expected`
// (paths relative to `dir`). This way mods removed in the new pack
// version disappear from the client.
fn prune_managed_dir(dir: &Path, expected: &HashSet<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let existing = collect_relative_files(dir);
    for rel in existing.difference(expected) {
        let full = dir.join(rel);
        if full.exists() {
            fs::remove_file(&full)
                .with_context(|| format!("Could not delete {}", full.display()))?;
        }
    }
    remove_empty_dirs(dir);
    Ok(())
}

fn extract_zip_flat(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = File::open(zip_path)
        .with_context(|| format!("Could not open the modpack zip: {}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = match entry.enclosed_name() {
            Some(n) => n.to_path_buf(),
            None => continue, // ruta insegura dentro del zip, se ignora
        };
        let out_path = dest.join(&name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    Ok(())
}

fn copy_dir_overwrite(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_overwrite(&path, &target)?;
        } else {
            fs::copy(&path, &target)
                .with_context(|| format!("Could not copy {}", path.display()))?;
        }
    }
    Ok(())
}

// Scans `mods_dir` for jars the user has disabled (".jar.disabled") and
// returns the set of their Forge mod ids. Reading the mod id (instead of just
// the file name) is what lets us recognize the same mod again later even if
// the modpack update ships it under a different jar file name because of a
// version bump (e.g. DistantHorizons-3.0.3-... -> DistantHorizons-3.1.2-...).
fn collect_disabled_mod_ids(mods_dir: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    if !mods_dir.exists() {
        return ids;
    }
    let Ok(entries) = fs::read_dir(mods_dir) else { return ids };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !name.to_lowercase().ends_with(".jar.disabled") {
            continue;
        }
        if let Ok(info) = read_mod_info_optimized(&path) {
            if !info.mod_id.trim().is_empty() {
                ids.insert(info.mod_id);
            }
        }
    }
    ids
}

// After the modpack has been synced, re-disables any mod jar whose mod id is
// in `disabled_mod_ids`. A mod that was disabled before the update always
// comes back from `copy_dir_overwrite` as a plain, enabled ".jar" (the
// modpack ships mods enabled), so without this step every update would
// silently re-enable mods the user had turned off on purpose.
fn reapply_disabled_mods(mods_dir: &Path, disabled_mod_ids: &HashSet<String>) -> Result<()> {
    if disabled_mod_ids.is_empty() || !mods_dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(mods_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if !name.to_lowercase().ends_with(".jar") {
            continue;
        }
        let Ok(info) = read_mod_info_optimized(&path) else { continue };
        if info.mod_id.trim().is_empty() || !disabled_mod_ids.contains(&info.mod_id) {
            continue;
        }
        let disabled_path = path.with_file_name(format!("{name}.disabled"));
        fs::rename(&path, &disabled_path)
            .with_context(|| format!("Could not re-disable {}", path.display()))?;
        eprintln!("Re-disabled updated mod (was off before the update): {}", name);
    }
    Ok(())
}

// Enforces the "Modpack Profile" choice on whatever Distant Horizons jar is
// currently installed (found by mod id, or by file name prefix as a
// fallback), regardless of its exact version/filename:
// - "no_dh": make sure it's disabled.
// - anything else ("default"): make sure it's enabled.
// A profile of "no_dh" is re-applied after every modpack sync too, so an
// update that ships Distant Horizons freshly enabled doesn't silently
// override the user's choice.
fn apply_modpack_profile(instance_dir: &Path, profile: &str) -> Result<()> {
    let mods_dir = instance_dir.join("mods");
    if !mods_dir.exists() {
        return Ok(());
    }
    let want_disabled = profile == "no_dh";

    let entries = fs::read_dir(&mods_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let raw_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let lower = raw_name.to_lowercase();
        if !(lower.ends_with(".jar") || lower.ends_with(".jar.disabled")) {
            continue;
        }

        let mod_id = read_mod_info_optimized(&path).map(|info| info.mod_id).unwrap_or_default();
        if !is_distant_horizons(&raw_name, &mod_id) {
            continue;
        }

        let currently_enabled = !lower.ends_with(".disabled");
        if want_disabled && currently_enabled {
            let disabled_path = path.with_file_name(format!("{raw_name}.disabled"));
            fs::rename(&path, &disabled_path)
                .with_context(|| format!("Could not disable {}", path.display()))?;
        } else if !want_disabled && !currently_enabled {
            let enabled_name = raw_name
                .strip_suffix(".disabled")
                .or_else(|| raw_name.strip_suffix(".DISABLED"))
                .unwrap_or(&raw_name)
                .to_string();
            let enabled_path = path.with_file_name(enabled_name);
            fs::rename(&path, &enabled_path)
                .with_context(|| format!("Could not enable {}", path.display()))?;
        }
    }
    Ok(())
}

fn modpack_profile_store_key() -> &'static str {
    "modpack_profile"
}

fn read_modpack_profile(app: &AppHandle) -> String {
    let Ok(store) = StoreBuilder::new(app, PathBuf::from("settings.json")).build() else {
        return "default".to_string();
    };
    store
        .get(modpack_profile_store_key())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "default".to_string())
}

// -----------------------------------------------
// Modpack Profile (Default / No DH)
// -----------------------------------------------
// Lets the user pick "No DH" to always keep Distant Horizons disabled on
// this machine, persisted independently of the regular settings so it
// survives app restarts and modpack updates until the user changes it back.
#[tauri::command]
async fn get_modpack_profile(app: AppHandle) -> Result<String, String> {
    Ok(read_modpack_profile(&app))
}

#[tauri::command]
async fn set_modpack_profile(app: AppHandle, profile: String) -> Result<(), String> {
    let profile = if profile == "no_dh" { "no_dh".to_string() } else { "default".to_string() };

    let store = StoreBuilder::new(&app, PathBuf::from("settings.json"))
        .build()
        .map_err(|e| e.to_string())?;
    store.set(modpack_profile_store_key().to_string(), serde_json::json!(profile));
    store.save().map_err(|e| e.to_string())?;

    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    apply_modpack_profile(&instance_dir, &profile).map_err(|e| e.to_string())?;

    Ok(())
}

/// Checks whether there's a modpack version different from the installed one,
/// WITHOUT downloading anything. Used to show the "Download Update" button.
#[tauri::command]
async fn check_modpack_update(app: AppHandle) -> Result<ModpackInfo, String> {
    let client = reqwest::Client::new();
    let release = fetch_latest_modpack_release(&client).await.map_err(|e| e.to_string())?;
    let asset = find_modpack_asset(&release).ok_or_else(|| {
        format!("El release '{}' no tiene un asset llamado '{}'", release.tag_name, MODPACK_ASSET_NAME)
    })?;

    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let state = read_modpack_state(&instance_dir);

    let up_to_date = state.tag_name == release.tag_name && state.asset_id == asset.id;

    Ok(ModpackInfo {
        version: release.tag_name.clone(),
        published_at: release.published_at.clone(),
        size: asset.size,
        up_to_date,
    })
}

/// Downloads (if needed) and syncs the modpack: adds/updates
/// files and DELETES ones that no longer belong, across mods/config/defaultconfigs/shaderpacks/resourcepacks.
#[tauri::command]
async fn sync_modpack(app: AppHandle, progress_channel: Channel<InstallProgress>) -> Result<String, String> {
    let client = reqwest::Client::new();

    send_progress(&progress_channel, "Checking modpack", 0, 1, 0.0, "Checking latest version...").await;
    let release = fetch_latest_modpack_release(&client).await.map_err(|e| e.to_string())?;
    let asset = find_modpack_asset(&release)
        .ok_or_else(|| format!("Release '{}' doesn't have '{}'", release.tag_name, MODPACK_ASSET_NAME))?
        .clone();

    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let shared_dir = get_shared_dir(&app_dir);
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let state = read_modpack_state(&instance_dir);

    if state.tag_name == release.tag_name && state.asset_id == asset.id {
        eprintln!("Modpack already up to date ({})", release.tag_name);
        return Ok(release.tag_name);
    }

    // 1. Download the zip
    let cache_dir = shared_dir.join("cache").join("modpack");
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let zip_path = cache_dir.join(format!("modpack-{}.zip", release.tag_name));

    send_progress(&progress_channel, "Downloading modpack", 0, 1, 0.0, &format!("Downloading {}...", release.tag_name)).await;
    download_file(&client, &asset.browser_download_url, &zip_path, None, Some(&progress_channel), "Downloading modpack", &asset.name)
        .await
        .map_err(|e| e.to_string())?;

    if !is_file_valid(&zip_path) {
        return Err("The downloaded modpack zip is empty or corrupt".to_string());
    }

    // 2. Extract to a temp folder
    send_progress(&progress_channel, "Extracting modpack", 0, 1, 0.0, "Extracting...").await;
    let temp_extract = std::env::temp_dir().join(format!("bps_modpack_{}", release.tag_name));
    if temp_extract.exists() {
        fs::remove_dir_all(&temp_extract).map_err(|e| e.to_string())?;
    }
    extract_zip_flat(&zip_path, &temp_extract).map_err(|e| e.to_string())?;

    // 3. Sync each managed folder (delete the old, copy the new)
    // Remember, by mod id, which mods the user had disabled BEFORE the sync so
    // we can re-disable them afterwards even if the update replaced their jar
    // with a new version/filename (mods always come back from the pack enabled).
    let disabled_mod_ids = collect_disabled_mod_ids(&instance_dir.join("mods"));

    let total_dirs = MODPACK_MANAGED_DIRS.len() as u64;
    for (idx, dir_name) in MODPACK_MANAGED_DIRS.iter().enumerate() {
        let expected = collect_relative_files(&temp_extract.join(dir_name));
        prune_managed_dir(&instance_dir.join(dir_name), &expected).map_err(|e| e.to_string())?;
        copy_dir_overwrite(&temp_extract.join(dir_name), &instance_dir.join(dir_name)).map_err(|e| e.to_string())?;
        send_progress(
            &progress_channel,
            "Syncing modpack",
            (idx + 1) as u64,
            total_dirs,
            ((idx + 1) as f32 / total_dirs as f32) * 100.0,
            dir_name,
        ).await;
    }

    // 3.5 Restore the mods the user had disabled, and re-apply the chosen
    // Modpack Profile (e.g. "No DH" keeps Distant Horizons off even if the
    // update shipped it enabled again).
    send_progress(&progress_channel, "Applying mod preferences", 0, 1, 99.0, "Restoring your mod preferences...").await;
    let mods_dir = instance_dir.join("mods");
    reapply_disabled_mods(&mods_dir, &disabled_mod_ids).map_err(|e| e.to_string())?;
    let profile = read_modpack_profile(&app);
    apply_modpack_profile(&instance_dir, &profile).map_err(|e| e.to_string())?;

    // 4. Cleanup and save state
    fs::remove_dir_all(&temp_extract).ok();
    let new_state = ModpackState {
        tag_name: release.tag_name.clone(),
        asset_id: asset.id,
        updated_at: asset.updated_at.clone(),
    };
    write_modpack_state(&instance_dir, &new_state).map_err(|e| e.to_string())?;

    eprintln!("Modpack synced to version {}", release.tag_name);
    Ok(release.tag_name)
}

// -----------------------------------------------
// show_instance_in_folder
// -----------------------------------------------
// Opens the modpack instance's root folder (where mods,
// config, resourcepacks, shaderpacks, saves, etc. live).
#[tauri::command]
async fn show_instance_in_folder(app: AppHandle) -> Result<(), String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));

    if !instance_dir.exists() {
        return Err("The instance folder doesn't exist yet. Install the modpack first.".to_string());
    }

    open_folder(&instance_dir)
}

// -----------------------------------------------
// is_game_running
// -----------------------------------------------
// Checks whether a Minecraft process is active (based on the PID stored in
// GameProcessState). It's cheap: it doesn't scan system processes, it just reads
// the in-memory state the launcher already keeps.
#[tauri::command]
async fn is_game_running(app: AppHandle) -> Result<bool, String> {
    let state = app
        .try_state::<GameProcessState>()
        .ok_or_else(|| "Could not read the game process state".to_string())?;
    let pid = *state.pid.lock().unwrap();
    Ok(pid.is_some())
}

async fn warm_mod_cache(app: &AppHandle) -> Result<()> {
    let app_dir = get_app_dir(app)?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let mods_dir = instance_dir.join("mods");

    if !mods_dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(&mods_dir)?;
    let jar_paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.ends_with(".jar") || name.ends_with(".jar.disabled")
        })
        .collect();

    if jar_paths.is_empty() {
        return Ok(());
    }

    let mut cache = HashMap::new();
    for path in jar_paths {
        let key = path
            .strip_prefix(&mods_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let (size, modified) = file_fingerprint_meta(&path).unwrap_or((0, 0));
        let info = read_mod_info_optimized(&path)?;
        cache.insert(key, ModCacheEntry { size, modified, info });
    }

    let cache_path = mod_info_cache_path(app)?;
    save_mod_info_cache_atomic(&cache_path, &cache)?;
    Ok(())
}

// -----------------------------------------------
// Other commands
// -----------------------------------------------

#[tauri::command]
async fn get_latest_log(app: AppHandle) -> Result<String, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let log_path = instance_dir.join("logs").join("latest.log");

    if !log_path.exists() {
        return Ok("No log file found. Start the game to generate logs.".to_string());
    }

    let bytes = fs::read(&log_path).map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Ok("Log file is empty.".to_string());
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tauri::command]
async fn list_log_files(app: AppHandle) -> Result<Vec<String>, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));
    let mut files = Vec::new();

    let latest = instance_dir.join("logs").join("latest.log");
    if latest.exists() {
        files.push("latest.log".to_string());
    }

    let logs_dir = instance_dir.join("logs");
    if logs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&logs_dir) {
            let mut gz_files: Vec<String> = entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".log.gz") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
            gz_files.sort_by(|a, b| b.cmp(a));
            files.extend(gz_files);
        }
    }

    let crash_dir = instance_dir.join("crash-reports");
    if crash_dir.exists() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let mut crash_files: Vec<String> = entries
                .filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("crash-") && name.ends_with(".txt") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();
            crash_files.sort_by(|a, b| b.cmp(a));
            files.extend(crash_files);
        }
    }

    Ok(files)
}

#[tauri::command]
async fn read_log_file(app: AppHandle, filename: String) -> Result<String, String> {
    let app_dir = get_app_dir(&app).map_err(|e| e.to_string())?;
    let instance_dir = get_instance_dir(&get_instances_dir(&app_dir));

    let path = if filename == "latest.log" {
        instance_dir.join("logs").join("latest.log")
    } else if filename.ends_with(".log.gz") {
        instance_dir.join("logs").join(&filename)
    } else if filename.starts_with("crash-") && filename.ends_with(".txt") {
        instance_dir.join("crash-reports").join(&filename)
    } else {
        return Err("Invalid filename".to_string());
    };

    if !path.exists() {
        return Ok("File not found.".to_string());
    }

    if filename.ends_with(".gz") {
        let file = File::open(&path).map_err(|e| e.to_string())?;
        let mut decoder = GzDecoder::new(file);
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[tauri::command]
async fn fetch_curseforge_mod_data(
    app: AppHandle,
    fingerprints: Vec<u32>,
) -> Result<Vec<CfModMeta>, String> {
    let map = get_cf_metadata(&app, &fingerprints)
        .await
        .map_err(|e| e.to_string())?;
    Ok(map.into_values().collect())
}

#[tauri::command]
async fn clear_mod_cache(app: AppHandle) -> Result<(), String> {
    let mod_info_path = mod_info_cache_path(&app).map_err(|e| e.to_string())?;
    let cf_path = cf_cache_path(&app).map_err(|e| e.to_string())?;

    if mod_info_path.exists() {
        fs::remove_file(&mod_info_path).map_err(|e| e.to_string())?;
    }
    if cf_path.exists() {
        fs::remove_file(&cf_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// -----------------------------------------------
// Entry point
// -----------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(GameProcessState::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            login_offline,
            login_microsoft,
            save_session,
            load_session,
            clear_session,
            scan_java,
            browse_java,
            browse_game_dir,
            save_settings,
            load_settings,
            launch_game_legacy,
            set_window_decorations,
            get_window_decorations,
            get_system_ram_mb,
            read_dir,
            file_exists,
            detect_java_25,
            play,
            stop_game,
            install_game,
            check_installation,
            get_instance_stats,
            reset_instance_playtime,
            list_java_installations,
            install_recommended_java,
            get_bundled_java_path,
            get_latest_log,
            list_log_files,
            read_log_file,
            list_installed_mods,
            fetch_curseforge_mod_data,
            set_mod_enabled,
            show_mod_in_folder,
            clear_mod_cache,
            list_content_items,
            set_content_item_enabled,
            show_content_item_in_folder,
            show_instance_in_folder,
            check_modpack_update,
            sync_modpack,
            get_modpack_profile,
            set_modpack_profile,
            is_game_running,
            fetch_changelogs,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Reaplicar la preferencia guardada de "native titlebar" apenas
            // arranca la app. Antes esto SOLO se hacía dentro de
            // save_settings() (por eso cambiar la opción en Configuración se
            // veía perfecto al instante), pero nunca se releía al iniciar la
            // app: la ventana principal siempre nacía con las decoraciones
            // por defecto de tauri.conf.json, ignorando lo que el usuario
            // había guardado. Por eso, al reabrir la app con "native
            // titlebar" activado, no aparecía ninguna barra de título.
            if let Ok(store) = StoreBuilder::new(&app_handle, PathBuf::from("settings.json")).build() {
                let custom_titlebar = store
                    .get("settings")
                    .and_then(|v| v.get("custom_titlebar").cloned())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.set_decorations(!custom_titlebar);
                }
            }

            tauri::async_runtime::spawn(async move {
                if let Ok(app_dir) = get_app_dir(&app_handle) {
                    let shared = get_shared_dir(&app_dir);
                    let instances = get_instances_dir(&app_dir);
                    let _ = fs::create_dir_all(&shared);
                    let _ = fs::create_dir_all(&instances);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}