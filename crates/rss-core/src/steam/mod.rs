//! Steam client session helpers. Real Steamworks behind `steam` feature.

use serde::{Deserialize, Serialize};

use crate::error::{RssError, RssResult};
use crate::RUST_APP_ID;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SteamMode {
    /// Uses local Steam client + steamworks SDK.
    Live,
    /// Deterministic mock for CI / offline demos.
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamStatus {
    pub mode: SteamMode,
    pub steam_running: bool,
    pub initialized: bool,
    pub app_id: u32,
    pub rust_owned: bool,
    pub steam_id: Option<String>,
    pub persona_name: Option<String>,
    pub messages: Vec<String>,
    pub ready_to_publish: bool,
}

pub trait SteamSession: Send {
    fn status(&self) -> SteamStatus;
    fn mode(&self) -> SteamMode;
}

/// Dry-run session that never talks to Steam.
pub struct MockSteamSession {
    pub steam_id: String,
    pub persona_name: String,
}

impl Default for MockSteamSession {
    fn default() -> Self {
        Self {
            steam_id: "76561198000000000".into(),
            persona_name: "DryRunUser".into(),
        }
    }
}

impl SteamSession for MockSteamSession {
    fn mode(&self) -> SteamMode {
        SteamMode::DryRun
    }

    fn status(&self) -> SteamStatus {
        SteamStatus {
            mode: SteamMode::DryRun,
            steam_running: true,
            initialized: true,
            app_id: RUST_APP_ID,
            rust_owned: true,
            steam_id: Some(self.steam_id.clone()),
            persona_name: Some(self.persona_name.clone()),
            messages: vec![
                "Dry-run mode: no Steam client contact.".into(),
                "PublishedFileIds will be synthetic (9xxxxxxxxx).".into(),
            ],
            ready_to_publish: true,
        }
    }
}

pub fn doctor_mock() -> SteamStatus {
    MockSteamSession::default().status()
}

#[cfg(feature = "steam")]
mod live {
    use super::*;
    use steamworks::{AppId, Client};

    pub struct LiveSteamSession {
        client: Client,
    }

    impl LiveSteamSession {
        pub fn connect() -> RssResult<Self> {
            let client = Client::init_app(AppId(RUST_APP_ID)).map_err(|e| {
                RssError::Steam(format!(
                    "Steam init failed for AppID {RUST_APP_ID}: {e}. Ensure Steam is running and this account owns Rust."
                ))
            })?;
            Ok(Self { client })
        }

        pub fn client(&self) -> &Client {
            &self.client
        }

        pub fn run_callbacks(&self) {
            self.client.run_callbacks();
        }
    }

    impl SteamSession for LiveSteamSession {
        fn mode(&self) -> SteamMode {
            SteamMode::Live
        }

        fn status(&self) -> SteamStatus {
            let apps = self.client.apps();
            let user = self.client.user();
            let friends = self.client.friends();
            let rust_owned = apps.is_app_installed(AppId(RUST_APP_ID));
            let steam_id = user.steam_id().raw().to_string();
            let persona_name = friends.name();

            let mut messages = Vec::new();
            if !rust_owned {
                messages.push(
                    "Rust (252490) does not appear installed/owned on this Steam account.".into(),
                );
            } else {
                messages.push("Steam client connected; Rust ownership OK.".into());
            }
            messages.push(
                "If CreateItem fails, accept the Steam Workshop Legal Agreement in a browser."
                    .into(),
            );

            SteamStatus {
                mode: SteamMode::Live,
                steam_running: true,
                initialized: true,
                app_id: RUST_APP_ID,
                rust_owned,
                steam_id: Some(steam_id),
                persona_name: Some(persona_name),
                ready_to_publish: rust_owned,
                messages,
            }
        }
    }

    pub fn doctor_live() -> RssResult<SteamStatus> {
        let session = LiveSteamSession::connect()?;
        Ok(session.status())
    }
}

#[cfg(feature = "steam")]
pub use live::{doctor_live, LiveSteamSession};

pub fn doctor(mode: SteamMode) -> RssResult<SteamStatus> {
    match mode {
        SteamMode::DryRun => Ok(doctor_mock()),
        SteamMode::Live => {
            #[cfg(feature = "steam")]
            {
                doctor_live()
            }
            #[cfg(not(feature = "steam"))]
            {
                Err(RssError::Steam(
                    "live Steam mode requires building with --features steam".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_doctor_ready() {
        let status = doctor(SteamMode::DryRun).unwrap();
        assert!(status.ready_to_publish);
        assert_eq!(status.mode, SteamMode::DryRun);
        assert_eq!(status.app_id, RUST_APP_ID);
    }
}
