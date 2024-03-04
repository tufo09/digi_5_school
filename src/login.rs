use std::{path::Path, sync::Arc};

use reqwest::ClientBuilder;
use reqwest_cookie_store::CookieStoreMutex;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncReadExt};

pub const BASE_URL: &str = "https://digi4school.at/";
pub const LOGIN_URL: &str = "https://digi4school.at/br/xhr/login";

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

pub async fn do_init_get(cookie_store: Arc<CookieStoreMutex>) -> anyhow::Result<()> {
    let client = ClientBuilder::new().cookie_provider(cookie_store).build()?;

    let response = client.get(BASE_URL).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Got non-success status code: {}",
            response.status()
        ));
    }

    Ok(())
}

pub async fn perform_login(
    client: &reqwest::Client,
    credentials: &Credentials,
) -> anyhow::Result<()> {
    let mut form: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    form.insert("email", &credentials.email);
    form.insert("password", &credentials.password);
    form.insert("indefinite", "1");

    let response = client.post(LOGIN_URL).form(&form).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Login failed"))
    }
}