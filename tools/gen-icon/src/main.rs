use std::fs::File;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/src-tauri/icons");
    let png_path = root.join("icon.png");
    let img = image::open(&png_path).expect("open icon.png").into_rgba8();

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 32, 48, 256] {
        let resized = image::imageops::resize(&img, size, size, image::imageops::FilterType::Lanczos3);
        let icon_image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image).expect("encode"));
    }

    let out = root.join("icon.ico");
    let mut file = File::create(&out).expect("create ico");
    icon_dir.write(&mut file).expect("write ico");
    println!("wrote {}", out.display());
}
