use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

pub async fn get_credentials(path: impl AsRef<Path>) -> anyhow::Result<Credentials> {
    let mut file = File::open(path).await?;
    let mut text = String::new();
    file.read_to_string(&mut text).await?;

    let credentials = serde_json::from_str(&text)?;

    Ok(credentials)
}
