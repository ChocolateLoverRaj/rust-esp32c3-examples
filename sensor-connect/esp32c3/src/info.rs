use common::Info;

pub fn get_info() -> Info {
    Info {
        name: env!("CARGO_PKG_AUTHORS").to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        homepage: env!("CARGO_PKG_HOMEPAGE").to_owned(),
        repository: env!("CARGO_PKG_REPOSITORY").to_owned(),
        authors: env!("CARGO_PKG_AUTHORS").to_owned(),
    }
}
