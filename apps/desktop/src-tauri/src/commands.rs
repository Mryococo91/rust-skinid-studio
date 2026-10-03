use std::fs;
use std::path::PathBuf;

use base64::Engine;
use chrono::Utc;
use rss_core::asset::{pack_asset, AssetMeta, DEFAULT_ITEM_TYPE};
use rss_core::image::{
    load_image_bytes, png_data_url, process_file, process_image, ImageProcessOptions, ImageValidation,
    RotateDegrees,
};
use rss_core::project::{
    create_project_dirs, ProjectRecord, ProjectStore, PublishHistoryEntry,
};
use rss_core::steam::{doctor, SteamMode, SteamStatus};
use rss_core::workshop::{
    create_publisher, PublishRequest, PublishResult, PublishVisibility,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ProcessOptionsDto {
    pub target_size: Option<u32>,
    pub rotate: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ProcessedImageDto {
    pub preview_data_url: String,
    pub validation: ImageValidation,
    pub png_base64: String,
}

#[derive(Debug, Deserialize)]
pub struct PrepareRequest {
    pub png_base64: String,
    pub title: String,
    pub description: String,
    pub item_type: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PrepareResponse {
    pub project_id: String,
    pub asset_dir: String,
    pub validation: ImageValidation,
    pub preview_data_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishDto {
    pub asset_dir: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub dry_run: bool,
    /// When true (default), always CreateItem → brand-new unique SkinID.
    /// When false, update `published_file_id` if provided.
    pub create_new: Option<bool>,
    pub published_file_id: Option<u64>,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PathsDto {
    pub data_dir: String,
    pub projects_dir: String,
}

fn rotate_from_degrees(deg: Option<u32>) -> RotateDegrees {
    match deg.unwrap_or(0) % 360 {
        90 => RotateDegrees::Deg90,
        180 => RotateDegrees::Deg180,
        270 => RotateDegrees::Deg270,
        _ => RotateDegrees::None,
    }
}

fn decode_b64(s: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

fn encode_b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn options_from_dto(dto: &ProcessOptionsDto) -> ImageProcessOptions {
    ImageProcessOptions {
        target_size: dto.target_size.unwrap_or(512),
        rotate: rotate_from_degrees(dto.rotate),
        ..Default::default()
    }
}

#[tauri::command]
pub fn process_image_path(
    path: String,
    options: ProcessOptionsDto,
) -> Result<ProcessedImageDto, String> {
    let processed = process_file(PathBuf::from(path).as_path(), &options_from_dto(&options))
        .map_err(|e| e.to_string())?;
    Ok(ProcessedImageDto {
        preview_data_url: png_data_url(&processed.png_bytes),
        validation: processed.validation,
        png_base64: encode_b64(&processed.png_bytes),
    })
}

#[tauri::command]
pub fn process_image_bytes(
    bytes_base64: String,
    options: ProcessOptionsDto,
) -> Result<ProcessedImageDto, String> {
    let raw = decode_b64(&bytes_base64)?;
    let img = load_image_bytes(&raw).map_err(|e| e.to_string())?;
    let processed = process_image(img, &options_from_dto(&options)).map_err(|e| e.to_string())?;
    Ok(ProcessedImageDto {
        preview_data_url: png_data_url(&processed.png_bytes),
        validation: processed.validation,
        png_base64: encode_b64(&processed.png_bytes),
    })
}

#[tauri::command]
pub fn prepare_asset(
    state: State<'_, AppState>,
    request: PrepareRequest,
) -> Result<PrepareResponse, String> {
    let png = decode_b64(&request.png_base64)?;
    let project_id = request
        .project_id
        .unwrap_or_else(ProjectStore::new_project_id);
    let asset_dir = create_project_dirs(&state.data_dir, &project_id).map_err(|e| e.to_string())?;
    let meta = AssetMeta {
        title: request.title.clone(),
        description: request.description.clone(),
        item_type: request
            .item_type
            .unwrap_or_else(|| DEFAULT_ITEM_TYPE.to_string()),
        tags: vec!["Version3".into(), "Skin".into()],
    };
    let packed = pack_asset(&asset_dir, &png, &meta).map_err(|e| e.to_string())?;

    let now = Utc::now();
    let record = ProjectRecord {
        id: project_id.clone(),
        title: request.title,
        description: request.description,
        item_type: meta.item_type,
        asset_dir: asset_dir.to_string_lossy().into(),
        published_file_id: packed.manifest.published_file_id.clone(),
        created_at: now,
        updated_at: now,
    };
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .upsert_project(&record)
        .map_err(|e| e.to_string())?;

    Ok(PrepareResponse {
        project_id,
        asset_dir: asset_dir.to_string_lossy().into(),
        validation: packed.validation,
        preview_data_url: png_data_url(&png),
    })
}

#[tauri::command]
pub fn steam_doctor(dry_run: bool) -> Result<SteamStatus, String> {
    let mode = if dry_run {
        SteamMode::DryRun
    } else {
        SteamMode::Live
    };
    doctor(mode).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn publish_asset(
    state: State<'_, AppState>,
    request: PublishDto,
) -> Result<PublishResult, String> {
    let packed = rss_core::asset::load_packed_asset(PathBuf::from(&request.asset_dir).as_path())
        .map_err(|e| e.to_string())?;
    let mode = if request.dry_run {
        SteamMode::DryRun
    } else {
        SteamMode::Live
    };
    let publisher = create_publisher(mode).map_err(|e| e.to_string())?;
    let visibility = match request.visibility.as_deref().unwrap_or("public") {
        "friends" | "friends_only" => PublishVisibility::FriendsOnly,
        "private" => PublishVisibility::Private,
        "unlisted" => PublishVisibility::Unlisted,
        _ => PublishVisibility::Public,
    };
    // Default: create a brand-new Workshop item (unique SkinID).
    // Never reuse a SkinID from the local manifest unless the user explicitly updates.
    let create_new = request.create_new.unwrap_or(true);
    let update_id = if create_new {
        None
    } else {
        request.published_file_id
    };

    let mut req = PublishRequest {
        title: request.title.clone(),
        description: request.description.clone(),
        tags: packed.meta.tags.clone(),
        visibility,
        change_note: Some(if update_id.is_some() {
            "Update from Rust SkinID Studio".into()
        } else {
            "Create unique SkinID from Rust SkinID Studio".into()
        }),
        published_file_id: update_id,
    };
    if req.title.is_empty() {
        req.title = packed.meta.title.clone();
    }

    let result = publisher
        .publish(&packed, &req, &mut |_| {})
        .map_err(|e| e.to_string())?;

    let store = state.store.lock().map_err(|e| e.to_string())?;
    if let Some(mut project) = store
        .get_project(&request.project_id)
        .map_err(|e| e.to_string())?
    {
        project.published_file_id = Some(result.published_file_id.to_string());
        project.title = request.title;
        project.description = request.description;
        project.updated_at = Utc::now();
        store.upsert_project(&project).map_err(|e| e.to_string())?;
    }
    store
        .add_history(&PublishHistoryEntry {
            id: Uuid::new_v4().to_string(),
            project_id: request.project_id,
            published_file_id: result.published_file_id.to_string(),
            url: result.url.clone(),
            dry_run: result.dry_run,
            success: true,
            message: result.messages.join(" | "),
            created_at: Utc::now(),
        })
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectRecord>, String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .list_projects()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ProjectRecord>, String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .get_project(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_history(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<PublishHistoryEntry>, String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .list_history(project_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_project(
    state: State<'_, AppState>,
    project: ProjectRecord,
) -> Result<(), String> {
    state
        .store
        .lock()
        .map_err(|e| e.to_string())?
        .upsert_project(&project)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn app_paths(state: State<'_, AppState>) -> Result<PathsDto, String> {
    Ok(PathsDto {
        data_dir: state.data_dir.to_string_lossy().into(),
        projects_dir: state.data_dir.join("projects").to_string_lossy().into(),
    })
}

/// Delete local projects, history DB rows, and on-disk project folders.
#[tauri::command]
pub fn wipe_local_data(state: State<'_, AppState>) -> Result<(), String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.wipe_all().map_err(|e| e.to_string())?;
    drop(store);

    let projects = state.data_dir.join("projects");
    if projects.is_dir() {
        fs::remove_dir_all(&projects).map_err(|e| e.to_string())?;
        fs::create_dir_all(&projects).map_err(|e| e.to_string())?;
    }
    Ok(())
}
