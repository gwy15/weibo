#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Reqwest error: {}", _0)]
    Network(#[fail(cause)] reqwest::Error),

    #[fail(display = "Json error: {}", _0)]
    Json(#[fail(cause)] serde_json::Error),

    #[fail(display = "Api error: {}", _0)]
    Api(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
