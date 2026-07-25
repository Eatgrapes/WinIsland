//! Privacy-preserving polling for local Codex Desktop session activity.
//!
//! Codex currently persists Desktop sessions as newline-delimited JSON below
//! `~/.codex/sessions`. This is an internal format, so callers should treat
//! this module as a best-effort activity indicator rather than a durable API.
//! The parser intentionally keeps only activity metadata and user-visible
//! assistant text. It never copies prompts, reasoning, tool arguments, or tool
//! output into a snapshot.

use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use windows::Win32::Storage::Packaging::Appx::{
    FindPackagesByPackageFamily, GetPackagePathByFullName, PACKAGE_FILTER_DIRECT,
    PACKAGE_FILTER_HEAD,
};
use windows::core::{PCWSTR, PWSTR};

const DISCOVERY_INTERVAL: Duration = Duration::from_secs(2);
const INITIAL_TAIL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BYTES_PER_POLL: u64 = 128 * 1024;
const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SESSION_DEPTH: usize = 3;
const DISPLAY_WINDOW: Duration = Duration::from_secs(30 * 60);
const MAX_PET_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_PET_SPRITESHEET_BYTES: u64 = 32 * 1024 * 1024;
const MAX_CODEX_LOGO_BYTES: u64 = 256 * 1024;
const MAX_ASAR_HEADER_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BUILT_IN_PETS: usize = 64;
const CODEX_PACKAGE_FAMILY: &str = "OpenAI.Codex_2p2nqsd0c76g0";
const CODEX_LOGO_PATHS: &[&str] = &[
    "assets/Square44x44Logo.targetsize-32_altform-unplated.png",
    "assets/Square44x44Logo.targetsize-44_altform-unplated.png",
    "assets/icon.png",
];

/// The coarse state that can be shown without exposing Codex session content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CodexState {
    #[default]
    Idle,
    Thinking,
    RunningTool,
    WaitingForUser,
    WaitingForApproval,
    Completed,
    Failed,
}

impl CodexState {
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Thinking | Self::RunningTool | Self::WaitingForUser | Self::WaitingForApproval
        )
    }
}

/// A privacy-filtered view of the most recently active local Codex session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodexSnapshot {
    pub state: CodexState,
    pub session_id: Option<String>,
    pub latest_assistant_message: Option<String>,
    pub active_subagents: usize,
}

/// A locally installed Codex v2 pet that can be rendered without bundling assets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexPet {
    pub id: String,
    pub display_name: String,
    asset: CodexPetAsset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CodexPetAsset {
    LocalFile {
        path: PathBuf,
        modified: Option<SystemTime>,
    },
    AsarEntry {
        archive_path: PathBuf,
        archive_modified: Option<SystemTime>,
        data_offset: u64,
        entry_offset: u64,
        length: u64,
    },
}

impl CodexPet {
    pub(crate) fn read_spritesheet_bytes(&self) -> io::Result<Vec<u8>> {
        match &self.asset {
            CodexPetAsset::LocalFile { path, .. } => read_local_spritesheet(path),
            CodexPetAsset::AsarEntry {
                archive_path,
                data_offset,
                entry_offset,
                length,
                ..
            } => read_asar_entry(archive_path, *data_offset, *entry_offset, *length),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PetManifest {
    #[serde(default)]
    id: Option<String>,
    display_name: String,
    sprite_version_number: u32,
    spritesheet_path: String,
}

#[derive(Default)]
struct BuiltInPetCache {
    archive_path: Option<PathBuf>,
    archive_modified: Option<SystemTime>,
    pets: Vec<CodexPet>,
}

static BUILT_IN_PET_CACHE: OnceLock<Mutex<BuiltInPetCache>> = OnceLock::new();

/// Polls local Codex Desktop session files without creating a background thread.
///
/// Call [`Self::poll`] from the WinIsland event loop. It returns `true` only
/// when the public snapshot changes. [`Self::try_poll`] is available to callers
/// that need to handle filesystem errors themselves.
pub struct CodexSessionMonitor {
    sessions_root: PathBuf,
    selected_path: Option<PathBuf>,
    cursor: Option<SessionCursor>,
    snapshot: CodexSnapshot,
    active_subagents: HashSet<String>,
    last_discovery: Option<Instant>,
    selected_modified: Option<SystemTime>,
}

#[derive(Default)]
struct SessionCursor {
    offset: u64,
    line_buffer: Vec<u8>,
    discarding_oversized_line: bool,
}

impl SessionCursor {
    fn push_byte(&mut self, byte: u8) -> Option<Vec<u8>> {
        if byte == b'\n' {
            if self.discarding_oversized_line {
                self.discarding_oversized_line = false;
                return None;
            }
            return Some(std::mem::take(&mut self.line_buffer));
        }

        if self.discarding_oversized_line {
            return None;
        }

        if self.line_buffer.len() >= MAX_LINE_BYTES {
            self.line_buffer.clear();
            self.discarding_oversized_line = true;
            return None;
        }

        self.line_buffer.push(byte);
        None
    }
}

impl Default for CodexSessionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexSessionMonitor {
    pub fn new() -> Self {
        Self::with_sessions_root(default_sessions_root())
    }

    pub fn with_sessions_root(sessions_root: impl Into<PathBuf>) -> Self {
        Self {
            sessions_root: sessions_root.into(),
            selected_path: None,
            cursor: None,
            snapshot: CodexSnapshot::default(),
            active_subagents: HashSet::new(),
            last_discovery: None,
            selected_modified: None,
        }
    }

    pub fn snapshot(&self) -> &CodexSnapshot {
        &self.snapshot
    }

    pub fn is_displayable(&self) -> bool {
        self.selected_path.is_some()
            && self.snapshot.state != CodexState::Idle
            && self
                .selected_modified
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .is_some_and(|age| age <= DISPLAY_WINDOW)
    }

    /// Poll while ignoring transient filesystem errors such as a session file
    /// being rotated between directory enumeration and opening it.
    pub fn poll(&mut self) -> bool {
        self.try_poll().unwrap_or(false)
    }

    pub fn try_poll(&mut self) -> io::Result<bool> {
        let previous = self.snapshot.clone();

        if self.should_discover() {
            let selected = newest_session_file(&self.sessions_root)?;
            self.last_discovery = Some(Instant::now());
            match selected {
                Some(path) if self.selected_path.as_ref() != Some(&path) => {
                    self.open_session(path)?;
                }
                Some(_) => {}
                None => self.clear_session(),
            }
        }

        if let Some(path) = self.selected_path.clone() {
            match self.read_new_records(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    self.clear_session();
                    self.last_discovery = None;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(self.snapshot != previous)
    }

    fn should_discover(&self) -> bool {
        self.last_discovery
            .is_none_or(|last| last.elapsed() >= DISCOVERY_INTERVAL)
    }

    fn open_session(&mut self, path: PathBuf) -> io::Result<()> {
        self.selected_path = Some(path.clone());
        self.cursor = Some(SessionCursor::default());
        self.active_subagents.clear();
        self.snapshot = CodexSnapshot {
            session_id: session_id_from_path(&path),
            ..CodexSnapshot::default()
        };

        let metadata = fs::metadata(&path)?;
        self.selected_modified = metadata.modified().ok();
        let file_length = metadata.len();
        let start = file_length.saturating_sub(INITIAL_TAIL_BYTES);
        let bytes = read_range(&path, start, file_length - start)?;
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.offset = start + bytes.len() as u64;
        }

        if start == 0 {
            self.process_bytes(&bytes);
        } else if let Some(first_newline) = bytes.iter().position(|byte| *byte == b'\n') {
            self.process_bytes(&bytes[first_newline + 1..]);
        }

        Ok(())
    }

    fn read_new_records(&mut self, path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        self.selected_modified = metadata.modified().ok();
        let file_length = metadata.len();
        let offset = self.cursor.as_ref().map_or(0, |cursor| cursor.offset);
        if file_length < offset {
            return self.open_session(path.to_path_buf());
        }

        let bytes_to_read = (file_length - offset).min(MAX_BYTES_PER_POLL);
        if bytes_to_read == 0 {
            return Ok(());
        }

        let bytes = read_range(path, offset, bytes_to_read)?;
        if let Some(cursor) = self.cursor.as_mut() {
            cursor.offset = cursor.offset.saturating_add(bytes.len() as u64);
        }
        self.process_bytes(&bytes);
        Ok(())
    }

    fn process_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let line = self
                .cursor
                .as_mut()
                .and_then(|cursor| cursor.push_byte(byte));
            if let Some(line) = line {
                self.process_record(&line);
            }
        }
    }

    fn process_record(&mut self, line: &[u8]) {
        // `Value` is scoped to this function. Do not log or retain raw records:
        // they can contain prompts, reasoning, tool inputs, and tool outputs.
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            return;
        };
        let Some(record_type) = record.get("type").and_then(Value::as_str) else {
            return;
        };
        let Some(payload) = record.get("payload").and_then(Value::as_object) else {
            return;
        };

        match record_type {
            "event_msg" => self.process_event(payload),
            "response_item" => self.process_response_item(payload),
            _ => {}
        }
    }

    fn process_event(&mut self, payload: &Map<String, Value>) {
        match string_field(payload, "type") {
            Some("task_started") => {
                self.snapshot.state = CodexState::Thinking;
                self.snapshot.latest_assistant_message = None;
                self.active_subagents.clear();
                self.update_subagent_count();
            }
            Some("agent_reasoning") => self.snapshot.state = CodexState::Thinking,
            Some("agent_message") => {
                if let Some(message) = string_field(payload, "message") {
                    self.set_assistant_message(message);
                }
            }
            Some("sub_agent_activity") => self.process_subagent_activity(payload),
            Some("task_complete") => {
                self.snapshot.state = if payload.get("error").is_some_and(|error| !error.is_null())
                {
                    CodexState::Failed
                } else {
                    CodexState::Completed
                };
                self.active_subagents.clear();
                self.update_subagent_count();
            }
            Some("turn_aborted") => self.snapshot.state = CodexState::Failed,
            _ => {}
        }
    }

    fn process_response_item(&mut self, payload: &Map<String, Value>) {
        match string_field(payload, "type") {
            Some("reasoning") => self.snapshot.state = CodexState::Thinking,
            Some("message") if string_field(payload, "role") == Some("assistant") => {
                if let Some(message) = assistant_output_text(payload) {
                    self.set_assistant_message(&message);
                }
            }
            Some("function_call") | Some("custom_tool_call") => {
                self.process_tool_call(payload);
            }
            Some("function_call_output") | Some("custom_tool_call_output")
                if self.snapshot.state == CodexState::RunningTool =>
            {
                self.snapshot.state = CodexState::Thinking;
            }
            _ => {}
        }
    }

    fn process_tool_call(&mut self, payload: &Map<String, Value>) {
        let name = string_field(payload, "name").unwrap_or_default();
        let status = string_field(payload, "status");

        if is_user_input_request(name) {
            self.snapshot.state = CodexState::WaitingForUser;
        } else if is_approval_request(name) {
            self.snapshot.state = CodexState::WaitingForApproval;
        } else if matches!(status, Some("failed")) {
            self.snapshot.state = CodexState::Failed;
        } else if matches!(status, Some("completed")) {
            self.snapshot.state = CodexState::Thinking;
        } else {
            self.snapshot.state = CodexState::RunningTool;
        }
    }

    fn process_subagent_activity(&mut self, payload: &Map<String, Value>) {
        let Some(agent_id) = string_field(payload, "agent_thread_id") else {
            return;
        };

        match string_field(payload, "kind") {
            Some("started") => {
                self.active_subagents.insert(agent_id.to_string());
            }
            Some("interrupted") | Some("completed") => {
                self.active_subagents.remove(agent_id);
            }
            _ => {}
        }
        self.update_subagent_count();
    }

    fn update_subagent_count(&mut self) {
        self.snapshot.active_subagents = self.active_subagents.len();
    }

    fn set_assistant_message(&mut self, message: &str) {
        if !message.is_empty() {
            self.snapshot.latest_assistant_message = Some(message.to_owned());
        }
    }

    fn clear_session(&mut self) {
        self.selected_path = None;
        self.cursor = None;
        self.active_subagents.clear();
        self.snapshot = CodexSnapshot::default();
        self.selected_modified = None;
    }
}

/// Returns the default local Codex Desktop session directory.
pub fn default_sessions_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("sessions")
}

pub(crate) fn read_local_codex_logo_bytes() -> io::Result<Vec<u8>> {
    let package_root = find_codex_package_root().or_else(|| {
        find_codex_asar_from_windows_apps().and_then(|archive_path| {
            archive_path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        })
    });
    let Some(package_root) = package_root else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not locate the installed Codex package",
        ));
    };

    for relative_path in CODEX_LOGO_PATHS {
        let path = package_root.join(relative_path);
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_CODEX_LOGO_BYTES {
            continue;
        }
        if let Ok(bytes) = fs::read(path) {
            return Ok(bytes);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Could not find a local Codex logo",
    ))
}

/// Discovers valid v2 pets from local packages and the installed Codex Desktop app.
///
/// User packages under `~/.codex/pets` override a same-named built-in pet. The
/// built-in spritesheets stay inside Codex's installed ASAR archive and are read
/// in place, so WinIsland does not bundle or copy Codex pet art.
pub fn discover_local_pets() -> Vec<CodexPet> {
    let mut pets = discover_user_pets();
    let mut known_ids = pets
        .iter()
        .map(|pet| pet.id.clone())
        .collect::<HashSet<_>>();
    for pet in discover_installed_codex_pets() {
        if known_ids.insert(pet.id.clone()) {
            pets.push(pet);
        }
    }
    pets.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then_with(|| left.id.cmp(&right.id))
    });
    pets
}

fn discover_user_pets() -> Vec<CodexPet> {
    let pets_root = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("pets");
    let Ok(entries) = fs::read_dir(pets_root) else {
        return Vec::new();
    };

    let mut pets = Vec::new();
    let mut known_ids = HashSet::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }

        let package_dir = entry.path();
        let Some(manifest) = read_pet_manifest(&package_dir.join("pet.json")) else {
            continue;
        };
        let Some(id) = manifest
            .id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .or_else(|| {
                entry
                    .file_name()
                    .into_string()
                    .ok()
                    .filter(|id| !id.trim().is_empty())
            })
        else {
            continue;
        };
        if manifest.sprite_version_number != 2
            || manifest.display_name.trim().is_empty()
            || known_ids.contains(&id)
        {
            continue;
        }

        let Some(spritesheet_path) =
            resolve_spritesheet_path(&package_dir, &manifest.spritesheet_path)
        else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&spritesheet_path) else {
            continue;
        };
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() == 0
            || metadata.len() > MAX_PET_SPRITESHEET_BYTES
            || !is_supported_spritesheet(&spritesheet_path)
        {
            continue;
        }

        known_ids.insert(id.clone());

        pets.push(CodexPet {
            id,
            display_name: manifest.display_name,
            asset: CodexPetAsset::LocalFile {
                path: spritesheet_path,
                modified: metadata.modified().ok(),
            },
        });
    }
    pets
}

fn discover_installed_codex_pets() -> Vec<CodexPet> {
    let archive_path = find_codex_asar();
    let archive_modified = archive_path
        .as_ref()
        .and_then(|path| fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok());
    let cache = BUILT_IN_PET_CACHE.get_or_init(|| Mutex::new(BuiltInPetCache::default()));
    if let Ok(cache) = cache.lock()
        && cache.archive_path == archive_path
        && cache.archive_modified == archive_modified
    {
        return cache.pets.clone();
    }

    let pets = archive_path
        .as_ref()
        .map(|path| discover_asar_pets(path, archive_modified))
        .unwrap_or_default();
    if let Ok(mut cache) = cache.lock() {
        cache.archive_path = archive_path;
        cache.archive_modified = archive_modified;
        cache.pets = pets.clone();
    }
    pets
}

fn find_codex_asar() -> Option<PathBuf> {
    find_codex_package_root()
        .map(|root| root.join("app").join("resources").join("app.asar"))
        .filter(|path| fs::metadata(path).is_ok_and(|metadata| metadata.is_file()))
        .or_else(find_codex_asar_from_windows_apps)
}

fn find_codex_package_root() -> Option<PathBuf> {
    let package_family = wide_null(CODEX_PACKAGE_FAMILY);
    let mut package_count = 0_u32;
    let mut buffer_length = 0_u32;
    // The sizing call normally returns ERROR_INSUFFICIENT_BUFFER. Its output
    // counts are still valid and are used for the allocation below.
    let _ = unsafe {
        FindPackagesByPackageFamily(
            PCWSTR::from_raw(package_family.as_ptr()),
            PACKAGE_FILTER_HEAD | PACKAGE_FILTER_DIRECT,
            &mut package_count,
            None,
            &mut buffer_length,
            None,
            None,
        )
    };
    if package_count == 0 || buffer_length == 0 {
        return None;
    }

    let mut package_full_names = vec![PWSTR::null(); usize::try_from(package_count).ok()?];
    let mut package_buffer = vec![0_u16; usize::try_from(buffer_length).ok()?];
    let result = unsafe {
        FindPackagesByPackageFamily(
            PCWSTR::from_raw(package_family.as_ptr()),
            PACKAGE_FILTER_HEAD | PACKAGE_FILTER_DIRECT,
            &mut package_count,
            Some(package_full_names.as_mut_ptr()),
            &mut buffer_length,
            Some(PWSTR::from_raw(package_buffer.as_mut_ptr())),
            None,
        )
    };
    if !result.is_ok() {
        return None;
    }

    package_full_names
        .into_iter()
        .take(usize::try_from(package_count).ok()?)
        .filter(|full_name| !full_name.is_null())
        .filter_map(|full_name| unsafe { full_name.to_string().ok() })
        .filter_map(|full_name| package_path_by_full_name(&full_name))
        .max_by_key(|path| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        })
}

fn package_path_by_full_name(full_name: &str) -> Option<PathBuf> {
    let full_name = wide_null(full_name);
    let mut path_length = 0_u32;
    // The sizing call normally returns ERROR_INSUFFICIENT_BUFFER.
    let _ = unsafe {
        GetPackagePathByFullName(PCWSTR::from_raw(full_name.as_ptr()), &mut path_length, None)
    };
    if path_length == 0 {
        return None;
    }

    let mut path = vec![0_u16; usize::try_from(path_length).ok()?];
    let result = unsafe {
        GetPackagePathByFullName(
            PCWSTR::from_raw(full_name.as_ptr()),
            &mut path_length,
            Some(PWSTR::from_raw(path.as_mut_ptr())),
        )
    };
    if !result.is_ok() {
        return None;
    }
    let length = path
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(path.len());
    String::from_utf16(&path[..length]).ok().map(PathBuf::from)
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn find_codex_asar_from_windows_apps() -> Option<PathBuf> {
    let mut roots = ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|path| PathBuf::from(path).join("WindowsApps"))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();

    let mut newest = None;
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_ascii_lowercase) else {
                continue;
            };
            if !name.starts_with("openai.codex_") {
                continue;
            }

            let archive_path = entry.path().join("app").join("resources").join("app.asar");
            let Ok(metadata) = fs::metadata(&archive_path) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() == 0 {
                continue;
            }
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let replace = newest
                .as_ref()
                .is_none_or(|(_, current_modified)| modified > *current_modified);
            if replace {
                newest = Some((archive_path, modified));
            }
        }
    }
    newest.map(|(path, _)| path)
}

fn discover_asar_pets(archive_path: &Path, archive_modified: Option<SystemTime>) -> Vec<CodexPet> {
    let Ok(archive_length) = fs::metadata(archive_path).map(|metadata| metadata.len()) else {
        return Vec::new();
    };
    let Some((data_offset, header)) = read_asar_header(archive_path, archive_length) else {
        return Vec::new();
    };
    let Some(asset_files) = header
        .get("files")
        .and_then(Value::as_object)
        .and_then(|files| files.get("webview"))
        .and_then(|entry| entry.get("files"))
        .and_then(Value::as_object)
        .and_then(|files| files.get("assets"))
        .and_then(|entry| entry.get("files"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    let mut pets = Vec::new();
    let mut known_ids = HashSet::new();
    collect_asar_pets(
        asset_files,
        archive_path,
        archive_modified,
        data_offset,
        archive_length,
        &mut known_ids,
        &mut pets,
    );
    pets
}

fn read_asar_header(path: &Path, archive_length: u64) -> Option<(u64, Value)> {
    let mut file = File::open(path).ok()?;
    let mut prefix = [0_u8; 16];
    file.read_exact(&mut prefix).ok()?;
    let header_size = u32::from_le_bytes(prefix[4..8].try_into().ok()?) as u64;
    let json_size = u32::from_le_bytes(prefix[12..16].try_into().ok()?) as u64;
    let data_offset = 8_u64.checked_add(header_size)?;
    if header_size > MAX_ASAR_HEADER_BYTES
        || json_size > MAX_ASAR_HEADER_BYTES
        || header_size != json_size.checked_add(8)?
        || data_offset > archive_length
        || 16_u64.checked_add(json_size)? > data_offset
    {
        return None;
    }

    let mut json = vec![0_u8; usize::try_from(json_size).ok()?];
    file.read_exact(&mut json).ok()?;
    serde_json::from_slice(&json)
        .ok()
        .map(|header| (data_offset, header))
}

#[allow(clippy::too_many_arguments)]
fn collect_asar_pets(
    files: &Map<String, Value>,
    archive_path: &Path,
    archive_modified: Option<SystemTime>,
    data_offset: u64,
    archive_length: u64,
    known_ids: &mut HashSet<String>,
    pets: &mut Vec<CodexPet>,
) {
    for (file_name, entry) in files {
        if pets.len() >= MAX_BUILT_IN_PETS {
            return;
        }
        if let Some(children) = entry.get("files").and_then(Value::as_object) {
            collect_asar_pets(
                children,
                archive_path,
                archive_modified,
                data_offset,
                archive_length,
                known_ids,
                pets,
            );
            continue;
        }
        if entry.get("unpacked").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(id) = built_in_pet_id(file_name) else {
            continue;
        };
        let Some(length) = entry.get("size").and_then(Value::as_u64) else {
            continue;
        };
        let Some(entry_offset) = entry.get("offset").and_then(asar_entry_offset) else {
            continue;
        };
        let Some(entry_end) = data_offset
            .checked_add(entry_offset)
            .and_then(|start| start.checked_add(length))
        else {
            continue;
        };
        if length == 0
            || length > MAX_PET_SPRITESHEET_BYTES
            || entry_end > archive_length
            || !known_ids.insert(id.to_string())
        {
            continue;
        }

        pets.push(CodexPet {
            id: id.to_string(),
            display_name: built_in_pet_display_name(id),
            asset: CodexPetAsset::AsarEntry {
                archive_path: archive_path.to_path_buf(),
                archive_modified,
                data_offset,
                entry_offset,
                length,
            },
        });
    }
}

fn built_in_pet_id(file_name: &str) -> Option<&str> {
    let stem = file_name
        .strip_suffix(".webp")
        .or_else(|| file_name.strip_suffix(".png"))?;
    let (id, _) = stem.split_once("-spritesheet-")?;
    (!id.is_empty()).then_some(id)
}

fn built_in_pet_display_name(id: &str) -> String {
    if id.eq_ignore_ascii_case("bsod") {
        return "BSOD".to_string();
    }
    id.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn asar_entry_offset(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse::<u64>().ok())
}

fn read_pet_manifest(path: &Path) -> Option<PetManifest> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > MAX_PET_MANIFEST_BYTES
    {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn resolve_spritesheet_path(package_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    let relative_path = Path::new(relative_path);
    let mut components = relative_path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return None;
    }
    Some(package_dir.join(relative_path))
}

fn is_supported_spritesheet(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("png") || extension.eq_ignore_ascii_case("webp")
        })
}

fn read_local_spritesheet(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_PET_SPRITESHEET_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid Codex pet spritesheet",
        ));
    }
    fs::read(path)
}

fn read_asar_entry(
    archive_path: &Path,
    data_offset: u64,
    entry_offset: u64,
    length: u64,
) -> io::Result<Vec<u8>> {
    if length == 0 || length > MAX_PET_SPRITESHEET_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid Codex ASAR pet spritesheet size",
        ));
    }
    let start = data_offset
        .checked_add(entry_offset)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid Codex ASAR offset"))?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid Codex ASAR range"))?;
    let metadata = fs::metadata(archive_path)?;
    if end > metadata.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Codex ASAR pet entry is outside the archive",
        ));
    }

    let capacity = usize::try_from(length)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Codex ASAR pet is too large"))?;
    let mut file = File::open(archive_path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = vec![0_u8; capacity];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

fn newest_session_file(root: &Path) -> io::Result<Option<PathBuf>> {
    let mut newest = None;
    visit_session_directory(root, 0, &mut newest)?;
    Ok(newest.map(|(path, _)| path))
}

fn visit_session_directory(
    directory: &Path,
    depth: usize,
    newest: &mut Option<(PathBuf, SystemTime)>,
) -> io::Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        if file_type.is_dir() && depth < MAX_SESSION_DEPTH {
            visit_session_directory(&path, depth + 1, newest)?;
            continue;
        }
        if !file_type.is_file() || !is_rollout_session(&path) {
            continue;
        }

        let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        let replace = newest
            .as_ref()
            .is_none_or(|(current_path, current_modified)| {
                modified > *current_modified
                    || (modified == *current_modified && path > *current_path)
            });
        if replace {
            *newest = Some((path, modified));
        }
    }

    Ok(())
}

fn is_rollout_session(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rollout-"))
}

fn read_range(path: &Path, offset: u64, length: u64) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::with_capacity(length.min(MAX_BYTES_PER_POLL) as usize);
    file.take(length).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn string_field<'a>(payload: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    payload.get(field).and_then(Value::as_str)
}

fn assistant_output_text(payload: &Map<String, Value>) -> Option<String> {
    let mut message = String::new();
    for part in payload.get("content").and_then(Value::as_array)? {
        if part.get("type").and_then(Value::as_str) != Some("output_text") {
            continue;
        }
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            message.push_str(text);
        }
    }
    (!message.is_empty()).then_some(message)
}

fn is_user_input_request(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized == "request_user_input" || normalized.ends_with(".request_user_input")
}

fn is_approval_request(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    normalized.contains("approval") || normalized.contains("request_confirmation")
}

fn session_id_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let start = stem.len().checked_sub(36)?;
    let id = stem.get(start..)?;
    id.chars()
        .all(|character| character.is_ascii_hexdigit() || character == '-')
        .then(|| id.to_string())
}
