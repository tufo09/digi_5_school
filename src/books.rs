use std::collections::HashMap;

use anyhow::Context;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{crawl::ParsedBook, util::ApiClient};

lazy_static! {
    static ref BOOK_HTML_TITLE_REGEX: Regex = Regex::new(r#"action='([^']+)'"#).unwrap();
    static ref BOOK_HTML_FORM_REGEX: Regex =
        Regex::new(r#"<input name='([^']+)' value='([^']*)'>"#).unwrap();
    static ref BOOK_HTML_META_REGEX: Regex =
        Regex::new(r#"<meta name="([^"]+)"(?: |\n)+content="([^"]*)" ?\/>"#).unwrap();
    static ref BOOK_HTML_PAGE_REGEX: Regex = Regex::new(r#"\[(\d+),(\d+)\]"#).unwrap();
    static ref IMG_REGEX: Regex = Regex::new(r#"#xlink:href="(img/[^"]+)"#).unwrap();
}

pub async fn do_book_form_dance(
    ApiClient(client, cookie_store): &ApiClient,
    url: &str,
) -> anyhow::Result<String> {
    let mut url = url.to_string();

    // Part 1: The html-form dance
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Got non-success status code: {}",
            response.status()
        ));
    };

    let text = response.text().await?;

    // println!("{text}");

    // 1a.1: Extract the url of the form
    BOOK_HTML_TITLE_REGEX
        .captures_iter(&text)
        .for_each(|capture| {
            url = capture[1].to_string();
            // println!("Found url: {}", url);
        });

    // 1a.2: Extract the form data
    let mut form = HashMap::new();

    BOOK_HTML_FORM_REGEX
        .captures_iter(&text)
        .for_each(|capture| {
            form.insert(capture[1].to_string(), capture[2].to_string());
        });

    dbg!(&url);
    let response = client.post(&url).form(&form).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Got non-success status code: {}",
            response.status()
        ));
    };

    let text = response.text().await?;

    // println!("\n{text}");

    // 1b.1: Extract the url of the form
    BOOK_HTML_TITLE_REGEX
        .captures_iter(&text)
        .for_each(|capture| {
            url = capture[1].to_string();
            // println!("Found url: {}", url);
        });

    // 1b.2: Extract the form data
    let mut form = HashMap::new();

    BOOK_HTML_FORM_REGEX
        .captures_iter(&text)
        .for_each(|capture| {
            form.insert(capture[1].to_string(), capture[2].to_string());
        });

    dbg!(&url);
    let response = client.post(&url).form(&form).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Got non-success status code: {}",
            response.status()
        ));
    };

    let text = response.text().await?;

    // println!("\n{text}");

    Ok(text)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Metadata parsed from the initial book html
pub struct BookMeta {
    title: String,
    sb_number: String,
    first_page: String,
    publisher: String,
    publisher_web: String,
    publisher_address: String,
    publisher_tel: String,
    publisher_mail: String,
    // viewport: String,
    page_sizes: Vec<[u16; 2]>,
}

pub fn extract_metadata_from_initial_html(initial_book_html: &str) -> anyhow::Result<BookMeta> {
    let mut meta = HashMap::new();

    // Get the <meta> tags
    BOOK_HTML_META_REGEX
        .captures_iter(initial_book_html)
        .for_each(|capture| {
            meta.insert(capture[1].to_string(), capture[2].to_string());
        });

    // Get the page sizes
    let mut page_sizes = Vec::new();

    BOOK_HTML_PAGE_REGEX
        .captures_iter(initial_book_html)
        .for_each(|capture| {
            let width = capture[1].parse::<u16>().unwrap();
            let height = capture[2].parse::<u16>().unwrap();

            page_sizes.push([width, height]);
        });

    let book_meta = BookMeta {
        title: meta.get("title").context("Missing title")?.to_string(),
        sb_number: meta.get("sbnr").context("Missing sbnr")?.to_string(),
        first_page: meta
            .get("firstPage")
            .context("Missing firstPage")?
            .to_string(),
        publisher: meta
            .get("publisher")
            .context("Missing publisher")?
            .to_string(),
        publisher_web: meta
            .get("publisherweb")
            .context("Missing publisherweb")?
            .to_string(),
        publisher_address: meta
            .get("publisheradr")
            .context("Missing publisheradr")?
            .to_string(),
        publisher_tel: meta
            .get("publishertel")
            .context("Missing publishertel")?
            .to_string(),
        publisher_mail: meta
            .get("publishermail")
            .context("Missing publishermail")?
            .to_string(),
        // viewport: meta
        //     .get("viewport")
        //     .context("Missing viewport")?
        //     .to_string(),
        page_sizes,
    };

    Ok(book_meta)
}

pub enum Version {
    Old,
    New,
}

pub async fn do_version_check(
    ApiClient(client, cookie_store): &ApiClient,
    book: &ParsedBook,
    book_meta: &BookMeta,
) -> anyhow::Result<Version> {
    let url = "https://a.digi4school.at/ebook/".to_string() + &book.id + "/" 
    // + &book.code + "/";
    ;
    let url = url.to_string() + "1/1.svg";

    dbg!(&url);
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        // return Err(anyhow::anyhow!(
        //     "Got non-success status code: {}",
        //     response.status()
        // ));

        dbg!(response.status());
        dbg!(response.text().await?);

        return Ok(Version::Old);
    };

    let text = response.text().await?;

    if text.contains("svg") {
        Ok(Version::Old)
    } else {
        Ok(Version::New)
    }
}

pub async fn do_download(
    ApiClient(client, cookie_store): &ApiClient,
    url: &str,
    book_meta: &BookMeta,
    version: &Version,
    book: &ParsedBook,
    save_path: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    // Append idx/idx.svg to the url, where idx is the page index
    for page in 1..=book_meta.page_sizes.len() {
        let url = format!("{url}{page}/{page}.svg");

        dbg!(&url);
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Got non-success status code: {}",
                response.status()
            ));
        };

        let text = response.text().await?;

        let mut path = save_path.as_ref().to_path_buf();
        path.push(format!("{page}.svg"));
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(text.as_bytes()).await?;
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Img {
    pub url: String,
    pub page_number: usize,
    pub img_number: usize,
}

pub async fn get_img(
    ApiClient(client, cookie_store): &ApiClient,
    book_meta: &BookMeta,
    version: &Version,
    book: &ParsedBook,
    svg_path: impl AsRef<std::path::Path>,
    img_path: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    let img_base_url = "https://a.digi4school.at/ebook/".to_string() + &book.id + "/";

    // Image urls containing the absolute path to the image
    let mut img_urls = Vec::new();

    // Iterate over all pages (svg files in the save_path)
    let files = std::fs::read_dir(svg_path.as_ref())?;

    for file in files {
        // Read the file
        let file = file?;
        let path = file.path();
        let text = tokio::fs::read_to_string(path.clone()).await?;

        // Extract the image urls
        IMG_REGEX.captures_iter(&text).for_each(|capture| {
            let relative_url = capture[1].to_string();
            // relative: img/1.png
            // absolute https://a.digi4school.at/ebook/1667/47/img/1.png

            // The page number is `x` in `x.svg`
            let page_number = path.file_stem().unwrap().to_str().unwrap();

            let url = format!("{img_base_url}{page_number}/{relative_url}");

            // `img_number` is the number in the relative url

            let img_number = relative_url
                .split("/")
                .last()
                .unwrap()
                .split(".")
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap();

            let img = Img {
                url,
                page_number: page_number.parse::<usize>().unwrap(),
                img_number: 0,
            };

            img_urls.push(img);
        });
    }

    dbg!(&img_urls[0..10]);

    // Download the images
    let path = img_path.as_ref().to_path_buf();
    for img in img_urls {
        let response = client.get(&img.url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Got non-success status code: {}",
                response.status()
            ));
        };

        let bytes = response.bytes().await?;

        let mut path = path.clone();
        path.push(format!(
            "{page_number}_{img_number}.png",
            page_number = img.page_number,
            img_number = img.img_number
        ));
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(&bytes).await?;
    }

    Ok(())
}
