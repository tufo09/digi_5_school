use std::{path::Path, sync::Arc};

use reqwest::{Client, ClientBuilder};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use tokio::{fs::File, io::AsyncReadExt};

pub struct ApiClient(pub Client, pub Arc<CookieStoreMutex>);

pub fn make_client_and_store() -> ApiClient {
    let cookie_store = CookieStore::default();
    let cookie_store = CookieStoreMutex::new(cookie_store);
    let cookie_store = Arc::new(cookie_store);

    let client = ClientBuilder::new()
        .cookie_provider(cookie_store.clone())
        .build()
        .unwrap();

    ApiClient(client, cookie_store)
}

pub async fn load_cookies_from_json(path: impl AsRef<Path>) -> anyhow::Result<ApiClient> {
    let cookie_store = {
        // TODO replace the `std::fs` calls with `tokio::fs` calls
        let mut file = File::open(path).await?;
        let mut text = String::new();
        file.read_to_string(&mut text).await?;

        reqwest_cookie_store::CookieStore::load_json(text.as_bytes()).unwrap()
    };
    let cookie_store = reqwest_cookie_store::CookieStoreMutex::new(cookie_store);
    let cookie_store = std::sync::Arc::new(cookie_store);

    let client = ClientBuilder::new()
        .cookie_provider(cookie_store.clone())
        .build()
        .unwrap();

    Ok(ApiClient(client, cookie_store))
}

pub fn make_dirs() {
    let dirs = [
        "d5s/keys/cookies",
        "d5s/keys/credentials",
        "d5s/downloads/meta",
        "d5s/downloads/assets",
        "d5s/downloads/pdfs",
        "d5s/downloads/images",
        "d5s/downloads/svgs",
        "d5s/downloads/pages",
    ];

    for dir in dirs {
        std::fs::create_dir_all(dir).unwrap();
    }
}
