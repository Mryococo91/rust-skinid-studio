use rss_core::asset::{build_manifest, pack_asset, AssetMeta, SkinManifest};
use rss_core::image::{process_image, ImageProcessOptions};
use image::{DynamicImage, Rgba, RgbaImage};
use tempfile::tempdir;

#[test]
fn golden_manifest_version3_grenade_empty_textures() {
    let manifest = build_manifest("Grenade");
    let json = manifest.to_pretty_json().unwrap();
    assert!(json.contains("\"Version\": 3"));
    assert!(json.contains("\"ItemType\": \"Grenade\""));
    assert!(json.contains("\"Textures\""));
    let parsed: SkinManifest = serde_json::from_str(&json).unwrap();
    assert!(parsed.groups[0].textures.is_empty());
}

#[test]
fn packed_asset_roundtrip_preserves_meta() {
    let dir = tempdir().unwrap();
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        700,
        500,
        Rgba([40, 80, 120, 255]),
    ));
    let processed = process_image(img, &ImageProcessOptions::default()).unwrap();
    let meta = AssetMeta {
        title: "Golden".into(),
        description: "test".into(),
        item_type: "Grenade".into(),
        tags: vec!["Version3".into(), "Skin".into()],
    };
    pack_asset(dir.path(), &processed.png_bytes, &meta).unwrap();
    let loaded = rss_core::asset::load_packed_asset(dir.path()).unwrap();
    assert_eq!(loaded.meta.title, "Golden");
    assert_eq!(loaded.manifest.item_type, "Grenade");
    assert!(loaded.validation.compatible);
}
