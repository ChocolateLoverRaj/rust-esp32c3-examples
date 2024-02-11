use serde::Serialize;

#[derive(Serialize)]
pub struct Info {
    pub name: &'static str,
    pub version: &'static str,
    pub homepage: &'static str,
    pub repository: &'static str,
    pub authors: &'static str,
}
pub const INFO: Info = Info {
    name: env!("CARGO_PKG_AUTHORS"),
    version: env!("CARGO_PKG_VERSION"),
    homepage: env!("CARGO_PKG_HOMEPAGE"),
    repository: env!("CARGO_PKG_REPOSITORY"),
    authors: env!("CARGO_PKG_AUTHORS"),
};
