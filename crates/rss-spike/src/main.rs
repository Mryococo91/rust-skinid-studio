//! Phase-0 spike: image → manifest pack → Workshop publish (dry-run by default).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rss_core::asset::{pack_asset, AssetMeta, DEFAULT_ITEM_TYPE};
use rss_core::image::{process_file, ImageProcessOptions};
use rss_core::steam::{doctor, SteamMode};
use rss_core::workshop::{create_publisher, PublishRequest, PublishVisibility};
use rss_core::{ALLOWED_ICON_SIZES, DEFAULT_ICON_SIZE};

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    DryRun,
    Live,
}

impl From<Mode> for SteamMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::DryRun => SteamMode::DryRun,
            Mode::Live => SteamMode::Live,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "rss-spike")]
#[command(about = "Rust Supply Signal Studio — Phase 0 spike CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check Steam readiness (dry-run always succeeds).
    Doctor {
        #[arg(long, value_enum, default_value_t = Mode::DryRun)]
        mode: Mode,
    },
    /// Prepare a Workshop content folder from an image (offline).
    Prepare {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "Custom Icon")]
        title: String,
        #[arg(long, default_value = "Custom icon-only Rust Workshop item (unique SkinID)")]
        description: String,
        #[arg(long, default_value = DEFAULT_ITEM_TYPE)]
        item_type: String,
        #[arg(long, default_value_t = DEFAULT_ICON_SIZE)]
        size: u32,
    },
    /// Publish a prepared folder (default: dry-run).
    Publish {
        #[arg(long)]
        asset_dir: PathBuf,
        #[arg(long, value_enum, default_value_t = Mode::DryRun)]
        mode: Mode,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        published_file_id: Option<u64>,
        #[arg(long, default_value = "public")]
        visibility: String,
    },
    /// One-shot: prepare + publish.
    Spike {
        #[arg(long)]
        image: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = Mode::DryRun)]
        mode: Mode,
        #[arg(long, default_value = "RSS Spike Custom Icon")]
        title: String,
        #[arg(long, default_value = DEFAULT_ITEM_TYPE)]
        item_type: String,
    },
}

fn parse_visibility(s: &str) -> Result<PublishVisibility> {
    match s.to_ascii_lowercase().as_str() {
        "public" => Ok(PublishVisibility::Public),
        "friends" | "friends_only" => Ok(PublishVisibility::FriendsOnly),
        "private" => Ok(PublishVisibility::Private),
        "unlisted" => Ok(PublishVisibility::Unlisted),
        other => bail!("unknown visibility '{other}'"),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor { mode } => {
            let status = doctor(mode.into())?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            if !status.ready_to_publish {
                bail!("Steam not ready to publish");
            }
        }
        Commands::Prepare {
            image,
            out,
            title,
            description,
            item_type,
            size,
        } => {
            if !ALLOWED_ICON_SIZES.contains(&size) {
                bail!("size must be one of {:?}", ALLOWED_ICON_SIZES);
            }
            std::fs::create_dir_all(&out)?;
            let opts = ImageProcessOptions {
                target_size: size,
                ..Default::default()
            };
            let processed = process_file(&image, &opts)
                .with_context(|| format!("process image {}", image.display()))?;
            let meta = AssetMeta {
                title,
                description,
                item_type,
                tags: vec!["Version3".into(), "Skin".into()],
            };
            let packed = pack_asset(&out, &processed.png_bytes, &meta)?;
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "root": packed.root,
                    "preview": packed.preview_path,
                    "content": packed.content_dir,
                    "validation": packed.validation,
                    "item_type": packed.manifest.item_type,
                })
            );
        }
        Commands::Publish {
            asset_dir,
            mode,
            title,
            published_file_id,
            visibility,
        } => {
            let packed = rss_core::asset::load_packed_asset(&asset_dir)?;
            let mut req = if let Some(id) = published_file_id {
                PublishRequest::for_update(&packed, id)
            } else {
                // Always create a unique SkinID unless --published-file-id is set.
                PublishRequest::from_asset(&packed)
            };
            if let Some(t) = title {
                req.title = t;
            }
            req.visibility = parse_visibility(&visibility)?;
            let publisher = create_publisher(mode.into())?;
            let result = publisher.publish(&packed, &req, &mut |p| {
                eprintln!("[{}%] {}: {}", p.percent, p.stage, p.detail);
            })?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Spike {
            image,
            out,
            mode,
            title,
            item_type,
        } => {
            std::fs::create_dir_all(&out)?;
            let processed = process_file(
                &image,
                &ImageProcessOptions {
                    target_size: DEFAULT_ICON_SIZE,
                    ..Default::default()
                },
            )?;
            let meta = AssetMeta {
                title: title.clone(),
                description: "Phase-0 spike: create a unique icon-only Workshop SkinID.".into(),
                item_type,
                tags: vec!["Version3".into(), "Skin".into()],
            };
            let packed = pack_asset(&out, &processed.png_bytes, &meta)?;
            eprintln!("Packed asset at {}", packed.root.display());
            let publisher = create_publisher(mode.into())?;
            let mut req = PublishRequest::from_asset(&packed);
            req.title = title;
            let result = publisher.publish(&packed, &req, &mut |p| {
                eprintln!("[{}%] {}: {}", p.percent, p.stage, p.detail);
            })?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            eprintln!(
                "\nNext: open {} — use this unique SkinID in your custom item / plugin config.",
                result.url
            );
        }
    }
    Ok(())
}
