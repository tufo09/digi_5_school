#![allow(dead_code, unused_variables)]

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use clap::{Parser, Subcommand};
use crawl::ParsedBook;
use reqwest_cookie_store::CookieStoreMutex;
use util::{make_dirs, ApiClient};

use crate::login::BASE_URL;

mod books;
mod crawl;
mod login;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();

    make_dirs();

    let cli = Cli::parse();

    match cli.command {
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
        Commands::CrawlInfo { book_metadata } => {
            handle_crawl_info(&timestamp, &book_metadata).await.unwrap();
        }
        Commands::GetBook {
            login_cookies,
            book_metadata,
            index,
        } => {
            handle_get_book(&timestamp, &login_cookies, &book_metadata, index)
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
    CrawlInfo {
        // #[clap(short, long)]
        /// The path to the JSON file containing the book metadata.
        /// (default d5s/downloads/meta/2023..._books.json)
        book_metadata: String,
    },
    GetBook {
        // #[clap(short, long)]
        /// The path to the JSON file containing the cookies after a successful login.
        /// (default d5s/keys/cookies/2023..._login.json)
        login_cookies: String,

        // #[clap(short, long)]
        /// The path to the JSON file containing the book metadata.
        /// (default d5s/downloads/meta/2023..._books.json)
        book_metadata: String,

        // #[clap(short, long)]
        /// The index of the book to download (starting at 0).
        index: usize,
    },
}

async fn handle_crawl_info(timestamp: &str, book_metadata: impl AsRef<Path>) -> anyhow::Result<()> {
    let books: Vec<ParsedBook> = serde_json::from_reader(std::fs::File::open(book_metadata)?)?;

    println!("Found {} books:", books.len());

    for (i, ParsedBook { title, .. }) in books.iter().enumerate() {
        // Use one space of left padding for the index
        println!("{i:>2}: {title}");
    }

    Ok(())
}

async fn handle_get_book(
    timestamp: &str,
    login_cookies: impl AsRef<Path>,
    book_metadata: impl AsRef<Path>,
    index: usize,
) -> anyhow::Result<()> {
    let books: Vec<ParsedBook> = serde_json::from_reader(std::fs::File::open(book_metadata)?)?;

    println!("Attempting to download book idx={index}...");

    let book = books.get(index).context("Invalid index")?;
    println!("Found book: {title}", title = book.title);

    let client = util::load_cookies_from_json(login_cookies).await?;

    let url = BASE_URL.to_string() + &book.url;
    let initial_book_html = books::do_book_form_dance(&client, &url).await.unwrap();

    write_cookies_to_disk(client.1.clone(), &timestamp, "do-book-form-dance")
        .await
        .unwrap();

    let mut path = PathBuf::from("d5s/downloads/meta");
    path.push(format!("initial_book_{id}_{timestamp}.html", id = book.id));
    let mut file = std::fs::File::create(path)?;
    file.write_all(initial_book_html.as_bytes())?;
    file.flush()?;

    println!("Wrote initial book html to disk.");

    let book_meta = books::extract_metadata_from_initial_html(&initial_book_html).unwrap();

    let mut path = PathBuf::from("d5s/downloads/meta");
    path.push(format!("book_meta_{id}_{timestamp}.json", id = book.id));
    let mut file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(&mut file, &book_meta)?;

    println!("Wrote book metadata to disk.");

    // let url = "https://a.digi4school.at/ebook/".to_string() + &book.id + "/" + &book.code + "/";

    dbg!(&url);
    let version = books::do_version_check(&client, &book, &book_meta)
        .await
        .unwrap();

    let url = "https://a.digi4school.at/ebook/".to_string() + &book.id + "/";

    Ok(())
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

#[deprecated]
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
