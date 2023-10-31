use std::collections::HashMap;

use anyhow::Context;
use bytes::Bytes;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{crawl::ParsedBook, util::ApiClient, BookComplete};

lazy_static! {
    static ref BOOK_HTML_TITLE_REGEX: Regex = Regex::new(r#"action='([^']+)'"#).unwrap();
    static ref BOOK_HTML_FORM_REGEX: Regex =
        Regex::new(r#"<input name='([^']+)' value='([^']*)'>"#).unwrap();
    static ref BOOK_HTML_META_REGEX: Regex =
        Regex::new(r#"<meta name="([^"]+)"(?: |\n)+content="([^"]*)" ?\/>"#).unwrap();
    static ref BOOK_HTML_PAGE_REGEX: Regex = Regex::new(r#"\[(\d+),(\d+)\]"#).unwrap();
    static ref IMG_REGEX: Regex = Regex::new(r#"xlink:href="(img/[^"]+)"#).unwrap();
    static ref SHADE_REGEX: Regex = Regex::new(r#"xlink:href="(shade/[^"]+)"#).unwrap();
}

pub async fn do_book_form_dance(
    ApiClient(client, _): &ApiClient,
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
    pub title: String,
    pub sb_number: String,
    pub first_page: String,
    pub publisher: String,
    pub publisher_web: String,
    pub publisher_address: String,
    pub publisher_tel: String,
    pub publisher_mail: String,
    // pub viewport: String,
    pub page_sizes: Vec<[u16; 2]>,
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

    // println!("{}", initial_book_html);

    let book_meta = BookMeta {
        title: meta.get("title").context("Missing title")?.to_string(),
        sb_number: meta.get("sbnr").context("Missing sbnr")?.to_string(),
        first_page: meta
            .get("firstPage")
            .unwrap_or(&"___missing_first_page".to_string())
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
    ApiClient(client, _): &ApiClient,
    book: &ParsedBook,
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
    ApiClient(client, _): &ApiClient,
    url: &str,
    book_meta: &BookMeta,
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
    pub img_type: ImgType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImgType {
    Img,
    Shade,
}

pub async fn get_img_urls(
    ApiClient(client, _): &ApiClient,
    // book_meta: &BookMeta,
    // version: &Version,
    // book: &ParsedBook,
    book_id: &str,
    svg_path: impl AsRef<std::path::Path>,
    // img_path: impl AsRef<std::path::Path>,
) -> anyhow::Result<Vec<Img>> {
    let img_base_url = "https://a.digi4school.at/ebook/".to_string() + &book_id + "/";
    // let img_base_url = "https://a.digi4school.at/ebook/".to_string() +  + "/";

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
                img_number,
                img_type: ImgType::Img,
            };

            img_urls.push(img);
        });

        // Extract the shade urls
        SHADE_REGEX.captures_iter(&text).for_each(|capture| {
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
                img_number,
                img_type: ImgType::Shade,
            };

            img_urls.push(img);
        });
    }

    dbg!(&img_urls[0..10]);

    Ok(img_urls)
}

pub async fn fetch_img(
    c: &ApiClient,
    // book_meta: &BookMeta,
    // version: &Version,
    // book: &ParsedBook,
    img_urls: &[Img],
    // svg_path: impl AsRef<std::path::Path>,
    img_path: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    let ApiClient(client, _) = &c;

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
            "{img_type}_{page_number}_{img_number}.png",
            img_type = get_img_name(&img.img_type),
            page_number = img.page_number,
            img_number = img.img_number
        ));
        let mut file = tokio::fs::File::create(path).await?;
        file.write_all(&bytes).await?;

        dbg!(&img.url);
    }

    Ok(())
}

const fn get_img_name(img_type: &ImgType) -> &'static str {
    match img_type {
        ImgType::Img => "img",
        ImgType::Shade => "shade",
    }
}

pub async fn dl_thumbnails(
    ApiClient(client, _): &ApiClient,
    book: &BookComplete,
    img_path: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    for page_number in 1..book.book_meta.page_sizes.len() {
        let url = format!("https://a.digi4school.at/ebook/1723/thumbnails/{page_number}.jpg");

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Got non-success status code: {}",
                response.status()
            ));
        };

        let bytes = response.bytes().await?;

        let mut path = img_path.as_ref().to_path_buf();
        path.push(format!("thumb_{page_number}.jpg",));

        dbg!(&path, &url);

        tokio::fs::File::create(path)
            .await?
            .write_all(&bytes)
            .await?;
    }

    Ok(())
}
