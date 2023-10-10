use std::sync::Arc;

use reqwest::{Client, ClientBuilder};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

pub fn make_client_and_store() -> (Client, Arc<CookieStoreMutex>) {
    let cookie_store = CookieStore::default();
    let cookie_store = CookieStoreMutex::new(cookie_store);
    let cookie_store = Arc::new(cookie_store);

    let client = ClientBuilder::new()
        .cookie_provider(cookie_store.clone())
        .build()
        .unwrap();

    (client, cookie_store)
}
