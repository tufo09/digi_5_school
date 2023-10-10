use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use reqwest_cookie_store::CookieStoreMutex;
use tokio::io::AsyncWriteExt;
use util::ApiClient;

mod login;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    // strategy_login(&timestamp).await?;

    Ok(())
}

async fn strategy_login(timestamp: &str) -> anyhow::Result<ApiClient> {
    let ApiClient(client, cookie_store) = util::make_client_and_store();

    let credentials = login::get_credentials("keys/credentials.json")
        .await
        .unwrap();

    write_cookies_to_disk(cookie_store.clone(), &timestamp, "empty")
        .await
        .unwrap();

    login::do_init_get(cookie_store.clone()).await.unwrap();

    write_cookies_to_disk(cookie_store.clone(), &timestamp, "init-get")
        .await
        .unwrap();

    login::perform_login(&client, &credentials).await.unwrap();

    write_cookies_to_disk(cookie_store.clone(), &timestamp, "login")
        .await
        .unwrap();

    Ok(ApiClient(client, cookie_store))
}

async fn strategy_load_cookies(
    timestamp: &str,
    path: impl AsRef<Path>,
) -> anyhow::Result<ApiClient> {
    let client = util::load_cookies_from_json(path).await?;

    write_cookies_to_disk(client.1.clone(), &timestamp, "load-cookies")
        .await
        .unwrap();

    Ok(client)
}

async fn write_cookies_to_disk(
    cookie_store: Arc<CookieStoreMutex>,
    timestamp: &str,
    name: &str,
) -> anyhow::Result<()> {
    let mut path = PathBuf::from("keys/cookies");
    path.push(format!("{timestamp}_{name}.json"));
    let mut file = tokio::fs::File::create(path).await?;

    let cookie_store = cookie_store.lock().unwrap();
    let cookies = serde_json::to_string_pretty(&*cookie_store).unwrap();

    file.write_all(cookies.as_bytes()).await?;

    Ok(())
}
