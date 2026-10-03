//! Workshop create / update publishing for Rust icon-only skins.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::asset::{write_published_file_id, PackedAsset};
use crate::error::{RssError, RssResult};
use crate::steam::{MockSteamSession, SteamMode, SteamSession};

#[cfg(feature = "steam")]
use std::path::Path;
#[cfg(feature = "steam")]
use crate::RUST_APP_ID;

pub const DEFAULT_TAGS: &[&str] = &["Version3", "Skin"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishVisibility {
    #[default]
    Public,
    FriendsOnly,
    Private,
    Unlisted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub visibility: PublishVisibility,
    pub change_note: Option<String>,
    /// If set, update existing item instead of creating a new one.
    pub published_file_id: Option<u64>,
}

impl PublishRequest {
    /// Build a request that always creates a **new** Workshop item (unique SkinID).
    pub fn from_asset(asset: &PackedAsset) -> Self {
        Self {
            title: asset.meta.title.clone(),
            description: asset.meta.description.clone(),
            tags: if asset.meta.tags.is_empty() {
                DEFAULT_TAGS.iter().map(|s| (*s).to_string()).collect()
            } else {
                asset.meta.tags.clone()
            },
            visibility: PublishVisibility::Public,
            change_note: Some("Create unique SkinID".into()),
            // Never auto-reuse a previous id — creation is the default product path.
            published_file_id: None,
        }
    }

    pub fn for_update(asset: &PackedAsset, published_file_id: u64) -> Self {
        let mut req = Self::from_asset(asset);
        req.published_file_id = Some(published_file_id);
        req.change_note = Some("Update existing SkinID".into());
        req
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishProgress {
    pub stage: String,
    pub percent: u32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub published_file_id: u64,
    pub url: String,
    pub dry_run: bool,
    pub needs_workshop_agreement: bool,
    pub messages: Vec<String>,
}

pub fn workshop_url(published_file_id: u64) -> String {
    format!("https://steamcommunity.com/sharedfiles/filedetails/?id={published_file_id}")
}

pub fn map_steam_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("legal") || lower.contains("agreement") {
        return "Steam Workshop Legal Agreement must be accepted in a browser before publishing."
            .into();
    }
    if lower.contains("not logged") || lower.contains("not connected") {
        return "Steam client is not running or not logged in.".into();
    }
    if lower.contains("file not found") || lower.contains("preview") {
        return "Preview or content path rejected by Steam. Check PNG path and size.".into();
    }
    raw.to_string()
}

/// Publisher abstraction used by CLI and Tauri.
pub trait WorkshopPublisher: Send {
    fn publish(
        &self,
        asset: &PackedAsset,
        request: &PublishRequest,
        on_progress: &mut dyn FnMut(PublishProgress),
    ) -> RssResult<PublishResult>;
}

pub struct MockWorkshopPublisher {
    session: MockSteamSession,
    next_id: Arc<Mutex<u64>>,
}

impl Default for MockWorkshopPublisher {
    fn default() -> Self {
        Self {
            session: MockSteamSession::default(),
            next_id: Arc::new(Mutex::new(9_000_000_001)),
        }
    }
}

impl WorkshopPublisher for MockWorkshopPublisher {
    fn publish(
        &self,
        asset: &PackedAsset,
        request: &PublishRequest,
        on_progress: &mut dyn FnMut(PublishProgress),
    ) -> RssResult<PublishResult> {
        let _ = self.session.status();
        if !asset.preview_path.exists() {
            return Err(RssError::Workshop("preview.png missing".into()));
        }
        if !asset.manifest_path.exists() {
            return Err(RssError::Workshop("manifest.txt missing".into()));
        }

        on_progress(PublishProgress {
            stage: "validating".into(),
            percent: 10,
            detail: "Dry-run validation".into(),
        });
        thread::sleep(Duration::from_millis(20));

        let id = if let Some(existing) = request.published_file_id {
            on_progress(PublishProgress {
                stage: "updating".into(),
                percent: 50,
                detail: format!("Updating existing item {existing}"),
            });
            existing
        } else {
            on_progress(PublishProgress {
                stage: "creating".into(),
                percent: 40,
                detail: "Creating synthetic Workshop item".into(),
            });
            let mut guard = self.next_id.lock().unwrap();
            let id = *guard;
            *guard += 1;
            id
        };

        on_progress(PublishProgress {
            stage: "submitting".into(),
            percent: 80,
            detail: "Mock submit".into(),
        });
        write_published_file_id(&asset.root, id)?;
        on_progress(PublishProgress {
            stage: "done".into(),
            percent: 100,
            detail: "Dry-run complete".into(),
        });

        Ok(PublishResult {
            published_file_id: id,
            url: workshop_url(id),
            dry_run: true,
            needs_workshop_agreement: false,
            messages: vec!["Dry-run publish succeeded (no Steam contact).".into()],
        })
    }
}

#[cfg(feature = "steam")]
mod live {
    use super::*;
    use crate::steam::LiveSteamSession;
    use std::sync::mpsc;
    use steamworks::{AppId, Client, FileType, PublishedFileId, PublishedFileVisibility};

    fn to_visibility(v: &PublishVisibility) -> PublishedFileVisibility {
        match v {
            PublishVisibility::Public => PublishedFileVisibility::Public,
            PublishVisibility::FriendsOnly => PublishedFileVisibility::FriendsOnly,
            // steamworks 0.12 has no Unlisted variant; map to Private.
            PublishVisibility::Private | PublishVisibility::Unlisted => {
                PublishedFileVisibility::Private
            }
        }
    }

    pub struct LiveWorkshopPublisher {
        session: LiveSteamSession,
    }

    impl LiveWorkshopPublisher {
        pub fn connect() -> RssResult<Self> {
            Ok(Self {
                session: LiveSteamSession::connect()?,
            })
        }

        fn client(&self) -> &Client {
            self.session.client()
        }

        fn wait_callback<T: Send + 'static>(
            &self,
            rx: mpsc::Receiver<RssResult<T>>,
            timeout_secs: u64,
        ) -> RssResult<T> {
            let started = std::time::Instant::now();
            loop {
                self.session.run_callbacks();
                match rx.try_recv() {
                    Ok(v) => return v,
                    Err(mpsc::TryRecvError::Empty) => {
                        if started.elapsed() > Duration::from_secs(timeout_secs) {
                            return Err(RssError::Workshop(
                                "timed out waiting for Steam callback".into(),
                            ));
                        }
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err(RssError::Workshop(
                            "Steam callback channel disconnected".into(),
                        ));
                    }
                }
            }
        }

        fn submit_update(
            &self,
            item_id: PublishedFileId,
            asset: &PackedAsset,
            request: &PublishRequest,
            on_progress: &mut dyn FnMut(PublishProgress),
        ) -> RssResult<(u64, bool)> {
            on_progress(PublishProgress {
                stage: "configuring".into(),
                percent: 55,
                detail: "Setting title, tags, preview, content".into(),
            });

            let (tx, rx) = mpsc::channel();
            let content = asset.content_dir.canonicalize().map_err(|e| {
                RssError::Workshop(format!("content path: {e}"))
            })?;
            let preview = asset.preview_path.canonicalize().map_err(|e| {
                RssError::Workshop(format!("preview path: {e}"))
            })?;

            let ugc = self.client().ugc();
            let update = ugc
                .start_item_update(AppId(RUST_APP_ID), item_id)
                .title(&request.title)
                .description(&request.description)
                .visibility(to_visibility(&request.visibility))
                .tags(request.tags.clone(), false)
                .content_path(Path::new(&content))
                .preview_path(Path::new(&preview));

            let note = request.change_note.clone();
            let _watch = update.submit(note.as_deref(), move |result| {
                let mapped = result
                    .map(|(id, needs)| (id.0, needs))
                    .map_err(|e| RssError::Workshop(map_steam_error(&e.to_string())));
                let _ = tx.send(mapped);
            });

            on_progress(PublishProgress {
                stage: "uploading".into(),
                percent: 75,
                detail: "Uploading to Steam Workshop".into(),
            });

            self.wait_callback(rx, 300)
        }
    }

    impl WorkshopPublisher for LiveWorkshopPublisher {
        fn publish(
            &self,
            asset: &PackedAsset,
            request: &PublishRequest,
            on_progress: &mut dyn FnMut(PublishProgress),
        ) -> RssResult<PublishResult> {
            let status = self.session.status();
            if !status.ready_to_publish {
                return Err(RssError::Steam(
                    status.messages.join(" ").if_empty("Steam not ready"),
                ));
            }

            on_progress(PublishProgress {
                stage: "validating".into(),
                percent: 5,
                detail: "Checking packed asset".into(),
            });

            let (item_id, created_needs_agreement) = if let Some(existing) = request.published_file_id
            {
                on_progress(PublishProgress {
                    stage: "updating".into(),
                    percent: 25,
                    detail: format!("Updating existing item {existing}"),
                });
                (PublishedFileId(existing), false)
            } else {
                on_progress(PublishProgress {
                    stage: "creating".into(),
                    percent: 20,
                    detail: "Creating Workshop item".into(),
                });
                let (tx, rx) = mpsc::channel();
                self.client().ugc().create_item(
                    AppId(RUST_APP_ID),
                    FileType::Community,
                    move |result| {
                        let mapped = result
                            .map(|(id, needs)| (id, needs))
                            .map_err(|e| RssError::Workshop(map_steam_error(&e.to_string())));
                        let _ = tx.send(mapped);
                    },
                );
                let (id, needs) = self.wait_callback(rx, 60)?;
                (id, needs)
            };

            let (final_id, submit_needs) =
                self.submit_update(item_id, asset, request, on_progress)?;
            let needs = created_needs_agreement || submit_needs;
            write_published_file_id(&asset.root, final_id)?;

            on_progress(PublishProgress {
                stage: "done".into(),
                percent: 100,
                detail: "Publish complete".into(),
            });

            let mut messages = Vec::new();
            if needs {
                messages.push(
                    "Accept the Steam Workshop Legal Agreement, then re-run publish/update."
                        .into(),
                );
            }
            messages.push(format!("SkinID: {final_id}"));

            Ok(PublishResult {
                published_file_id: final_id,
                url: workshop_url(final_id),
                dry_run: false,
                needs_workshop_agreement: needs,
                messages,
            })
        }
    }

    trait IfEmpty {
        fn if_empty(self, fallback: &str) -> String;
    }
    impl IfEmpty for String {
        fn if_empty(self, fallback: &str) -> String {
            if self.trim().is_empty() {
                fallback.to_string()
            } else {
                self
            }
        }
    }
}

#[cfg(feature = "steam")]
pub use live::LiveWorkshopPublisher;

pub fn create_publisher(mode: SteamMode) -> RssResult<Box<dyn WorkshopPublisher>> {
    match mode {
        SteamMode::DryRun => Ok(Box::new(MockWorkshopPublisher::default())),
        SteamMode::Live => {
            #[cfg(feature = "steam")]
            {
                Ok(Box::new(LiveWorkshopPublisher::connect()?))
            }
            #[cfg(not(feature = "steam"))]
            {
                Err(RssError::Steam(
                    "live publish requires building with --features steam".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::{pack_asset, AssetMeta};
    use crate::image::{process_image, ImageProcessOptions};
    use image::{DynamicImage, Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn dry_run_publish_writes_id() {
        let dir = tempdir().unwrap();
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(512, 512, Rgba([1, 2, 3, 255])));
        let processed = process_image(img, &ImageProcessOptions::default()).unwrap();
        let packed = pack_asset(dir.path(), &processed.png_bytes, &AssetMeta::default()).unwrap();
        let publisher = MockWorkshopPublisher::default();
        let req = PublishRequest::from_asset(&packed);
        let mut stages = Vec::new();
        let result = publisher
            .publish(&packed, &req, &mut |p| stages.push(p.stage.clone()))
            .unwrap();
        assert!(result.dry_run);
        assert!(result.published_file_id >= 9_000_000_001);
        assert!(!stages.is_empty());
        let reloaded = crate::asset::load_packed_asset(dir.path()).unwrap();
        assert_eq!(
            reloaded.manifest.published_file_id.unwrap(),
            result.published_file_id.to_string()
        );
    }
}
