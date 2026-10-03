use thiserror::Error;

pub type RssResult<T> = Result<T, RssError>;

#[derive(Debug, Error)]
pub enum RssError {
    #[error("image error: {0}")]
    Image(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("asset error: {0}")]
    Asset(String),

    #[error("steam error: {0}")]
    Steam(String),

    #[error("workshop error: {0}")]
    Workshop(String),

    #[error("project error: {0}")]
    Project(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("{0}")]
    Other(String),
}

impl From<image::ImageError> for RssError {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value.to_string())
    }
}

impl From<anyhow::Error> for RssError {
    fn from(value: anyhow::Error) -> Self {
        Self::Other(value.to_string())
    }
}
