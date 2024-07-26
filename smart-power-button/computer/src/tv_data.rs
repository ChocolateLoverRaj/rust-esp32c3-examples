use std::io::ErrorKind;

use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::config::TV_DATA_FILE;

// In case we need to add more to this later
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct TvData {
    pub is_on: bool,
    pub token: Option<String>,
}

pub async fn get_tv_data() -> anyhow::Result<Option<TvData>> {
    match File::open(TV_DATA_FILE).await {
        Ok(mut file) => {
            let mut buf = Default::default();
            file.read_to_end(&mut buf).await?;
            let tv_data: TvData = from_bytes(&buf)?;
            Ok(Some(tv_data))
        }
        Err(error) => match error.kind() {
            ErrorKind::NotFound => Ok(None),
            _ => Err(error.into()),
        },
    }
}

pub async fn save_tv_data(tv_data: &TvData) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(TV_DATA_FILE)
        .await?;
    file.write_all(&to_allocvec(tv_data)?).await?;
    Ok(())
}
