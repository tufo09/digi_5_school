mod login;

const BASE_URL: &str = "https://digi4school.at/";

#[tokio::main]
async fn main() {
    let credentials = login::get_credentials("keys/credentials.json")
        .await
        .unwrap();
}
