use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use reqwest_cookie_store::CookieStoreMutex;
use util::{make_dirs, ApiClient};

mod crawl;
mod login;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    make_dirs();

    let cli = Cli::parse();

    let client = match cli.command {
        Commands::Login { path } => {
            handle_login(&timestamp, &path).await.unwrap();
        }
        // Commands::Resume { login_cookies } => {
        //     handle_resume(&timestamp, &login_cookies).await.unwrap();
        // }
        Commands::CrawlBooks { login_cookies } => {
            handle_crawl_books(&timestamp, &login_cookies)
                .await
                .unwrap();
        }
    };

    Ok(())
}

#[derive(Parser)]
#[command()]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Login {
        // #[clap(short, long)]
        // email: String,

        // #[clap(short, long)]
        // password: String,
        /// The path to the JSON file containing the credentials ("email" and "password").
        path: String,
    },
    // Resume {
    //     // #[clap(short, long)]
    //     /// The path to the JSON file containing the cookies after a successful login.
    //     login_cookies: String,
    // },
    CrawlBooks {
        // #[clap(short, long)]
        /// The path to the JSON file containing the cookies after a successful login.
        /// (default d5s/keys/cookies/2023..._login.json)
        login_cookies: String,
    },
}

async fn handle_login(timestamp: &str, path: impl AsRef<Path>) -> anyhow::Result<ApiClient> {
    let ApiClient(client, cookie_store) = util::make_client_and_store();

    let credentials = login::get_credentials(path).await.unwrap();

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

    println!("Logged in successfully.");

    Ok(ApiClient(client, cookie_store))
}

async fn handle_resume(timestamp: &str, path: impl AsRef<Path>) -> anyhow::Result<ApiClient> {
    let client = util::load_cookies_from_json(path).await?;

    write_cookies_to_disk(client.1.clone(), &timestamp, "load-cookies")
        .await
        .unwrap();

    Ok(client)
}

async fn handle_crawl_books(timestamp: &str, path: impl AsRef<Path>) -> anyhow::Result<ApiClient> {
    let client = util::load_cookies_from_json(path).await?;

    write_cookies_to_disk(client.1.clone(), &timestamp, "load-cookies")
        .await
        .unwrap();

    let books = crawl::get_books(&client).await.unwrap();

    let mut path = PathBuf::from("d5s/downloads/meta");
    path.push(format!("{timestamp}_books.json"));
    let file = std::fs::File::create(path)?;
    let mut file = std::io::BufWriter::new(file);

    serde_json::to_writer_pretty(&mut file, &books)?;

    println!("Crawled books successfully.");

    Ok(client)
}

async fn write_cookies_to_disk(
    cookie_store: Arc<CookieStoreMutex>,
    timestamp: &str,
    name: &str,
) -> anyhow::Result<()> {
    let mut path = PathBuf::from("d5s/keys/cookies");
    path.push(format!("{timestamp}_{name}.json"));
    let file = std::fs::File::create(path)?;
    let mut file = std::io::BufWriter::new(file);

    let cookie_store = cookie_store.lock().unwrap();
    cookie_store.save_json(&mut file).unwrap();

    Ok(())
}
