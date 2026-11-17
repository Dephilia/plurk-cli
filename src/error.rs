#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid key file {path}: {source}")]
    ConfigParse {
        path: String,
        source: toml::de::Error,
    },
    #[error("cannot encode key file: {0}")]
    ConfigEncode(#[from] toml::ser::Error),
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("oauth request failed: {0}")]
    Oauth(#[from] reqwest_oauth1::Error),
    #[error("Plurk API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("cannot decode response ({source}): {body}")]
    Decode {
        source: serde_json::Error,
        body: String,
    },
    #[error("invalid url: {0}")]
    Url(String),
    #[error("invalid OAuth reply: {0}")]
    OAuthReply(String),
    #[error("comet offset out of sync")]
    Resync,
}

pub type Result<T> = std::result::Result<T, Error>;
