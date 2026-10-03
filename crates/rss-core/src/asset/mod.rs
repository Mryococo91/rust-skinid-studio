//! Rust Workshop icon-only asset packing (`manifest.txt` + icon).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{RssError, RssResult};
use crate::image::{validate_png_bytes, ImageValidation};

/// Workshop ItemType used for icon-only custom items (not an in-game item bind).
/// Rust needs a valid category in the manifest; this does NOT attach the SkinID
/// to a specific shortname like supply.signal — plugins do that later if needed.
pub const DEFAULT_ITEM_TYPE: &str = "Grenade";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestGroup {
    #[serde(rename = "Textures")]
    pub textures: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "Floats")]
    pub floats: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "Colors")]
    pub colors: serde_json::Map<String, serde_json::Value>,
}

impl Default for ManifestGroup {
    fn default() -> Self {
        Self {
            textures: serde_json::Map::new(),
            floats: serde_json::Map::new(),
            colors: serde_json::Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkinManifest {
    #[serde(rename = "Version")]
    pub version: u32,
    #[serde(rename = "ItemType")]
    pub item_type: String,
    #[serde(rename = "Groups")]
    pub groups: Vec<ManifestGroup>,
    /// Written back after a successful publish (local bookkeeping).
    #[serde(rename = "PublishedFileId", skip_serializing_if = "Option::is_none")]
    pub published_file_id: Option<String>,
}

impl SkinManifest {
    pub fn icon_only(item_type: impl Into<String>) -> Self {
        Self {
            version: 3,
            item_type: item_type.into(),
            groups: vec![ManifestGroup::default()],
            published_file_id: None,
        }
    }

    pub fn to_pretty_json(&self) -> RssResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMeta {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub item_type: String,
}

impl Default for AssetMeta {
    fn default() -> Self {
        Self {
            title: "Custom Icon".into(),
            description: "Custom icon-only Rust Workshop item (unique SkinID).".into(),
            tags: vec!["Version3".into(), "Skin".into()],
            item_type: DEFAULT_ITEM_TYPE.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackedAsset {
    pub root: PathBuf,
    pub content_dir: PathBuf,
    pub preview_path: PathBuf,
    pub manifest_path: PathBuf,
    pub icon_path: PathBuf,
    pub manifest: SkinManifest,
    pub meta: AssetMeta,
    pub validation: ImageValidation,
}

pub fn build_manifest(item_type: &str) -> SkinManifest {
    SkinManifest::icon_only(item_type)
}

pub fn pack_asset(
    output_dir: &Path,
    png_bytes: &[u8],
    meta: &AssetMeta,
) -> RssResult<PackedAsset> {
    let validation = validate_png_bytes(png_bytes)?;
    if !validation.compatible {
        return Err(RssError::Validation(format!(
            "icon is not compatible: {}",
            validation.messages.join("; ")
        )));
    }

    let content_dir = output_dir.join("content");
    fs::create_dir_all(&content_dir)?;

    let preview_path = output_dir.join("preview.png");
    let icon_path = content_dir.join("icon.png");
    let manifest_path = content_dir.join("manifest.txt");

    fs::write(&preview_path, png_bytes)?;
    fs::write(&icon_path, png_bytes)?;

    let manifest = build_manifest(&meta.item_type);
    fs::write(&manifest_path, manifest.to_pretty_json()?)?;

    // Persist meta alongside pack for republish.
    let meta_path = output_dir.join("meta.json");
    fs::write(&meta_path, serde_json::to_string_pretty(meta)?)?;

    Ok(PackedAsset {
        root: output_dir.to_path_buf(),
        content_dir,
        preview_path,
        manifest_path,
        icon_path,
        manifest,
        meta: meta.clone(),
        validation,
    })
}

pub fn load_packed_asset(output_dir: &Path) -> RssResult<PackedAsset> {
    let content_dir = output_dir.join("content");
    let preview_path = output_dir.join("preview.png");
    let icon_path = content_dir.join("icon.png");
    let manifest_path = content_dir.join("manifest.txt");
    let meta_path = output_dir.join("meta.json");

    let png_bytes = fs::read(&preview_path)?;
    let validation = validate_png_bytes(&png_bytes)?;
    let manifest: SkinManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    let meta: AssetMeta = if meta_path.exists() {
        serde_json::from_str(&fs::read_to_string(&meta_path)?)?
    } else {
        AssetMeta {
            item_type: manifest.item_type.clone(),
            ..AssetMeta::default()
        }
    };

    Ok(PackedAsset {
        root: output_dir.to_path_buf(),
        content_dir,
        preview_path,
        manifest_path,
        icon_path,
        manifest,
        meta,
        validation,
    })
}

pub fn write_published_file_id(asset_root: &Path, published_file_id: u64) -> RssResult<()> {
    let manifest_path = asset_root.join("content").join("manifest.txt");
    let mut manifest: SkinManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    manifest.published_file_id = Some(published_file_id.to_string());
    fs::write(&manifest_path, manifest.to_pretty_json()?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{process_image, ImageProcessOptions};
    use image::{DynamicImage, Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn manifest_icon_only_has_empty_textures() {
        let m = SkinManifest::icon_only("Grenade");
        assert_eq!(m.version, 3);
        assert_eq!(m.item_type, "Grenade");
        assert!(m.groups[0].textures.is_empty());
        let json = m.to_pretty_json().unwrap();
        assert!(json.contains("\"Version\": 3"));
        assert!(json.contains("\"ItemType\": \"Grenade\""));
    }

    #[test]
    fn pack_writes_expected_files() {
        let dir = tempdir().unwrap();
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(600, 400, Rgba([10, 20, 30, 255])));
        let processed = process_image(img, &ImageProcessOptions::default()).unwrap();
        let packed = pack_asset(dir.path(), &processed.png_bytes, &AssetMeta::default()).unwrap();
        assert!(packed.preview_path.exists());
        assert!(packed.manifest_path.exists());
        assert!(packed.icon_path.exists());
        assert!(packed.validation.compatible);
    }
}
