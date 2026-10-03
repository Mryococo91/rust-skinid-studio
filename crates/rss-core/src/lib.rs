//! Rust Supply Signal Studio — core library.
//!
//! Offline-capable image/asset/project modules. Steam/Workshop publishing is
//! available behind the `steam` feature, with a dry-run mock always available.

pub mod asset;
pub mod error;
pub mod image;
pub mod project;
pub mod steam;
pub mod workshop;

pub use error::{RssError, RssResult};

/// Facepunch Rust AppID on Steam.
pub const RUST_APP_ID: u32 = 252_490;

/// Default icon edge length (pixels).
pub const DEFAULT_ICON_SIZE: u32 = 512;

/// Allowed square icon sizes (power-of-two).
pub const ALLOWED_ICON_SIZES: &[u32] = &[256, 512, 1024, 2048];

/// Soft community max file size for preview icons (bytes).
pub const MAX_ICON_BYTES: u64 = 1_048_576;
