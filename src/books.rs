use std::collections::HashMap;

use anyhow::Context;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::util::ApiClient;

lazy_static! {
    static ref BOOK_HTML_TITLE_REGEX: Regex = Regex::new(r#"action='([^']+)'"#).unwrap();
    static ref BOOK_HTML_FORM_REGEX: Regex =
        Regex::new(r#"<input name='([^']+)' value='([^']*)'>"#).unwrap();
    static ref BOOK_HTML_META_REGEX: Regex =
        Regex::new(r#"<meta name="([^"]+)"(?: |\n)+content="([^"]*)" ?\/>"#).unwrap();
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
}

pub fn extract_metadata_from_initial_html(initial_book_html: &str) -> anyhow::Result<BookMeta> {
    let mut meta = HashMap::new();

    BOOK_HTML_META_REGEX
        .captures_iter(&initial_book_html)
        .for_each(|capture| {
            meta.insert(capture[1].to_string(), capture[2].to_string());
        });

    dbg!(&meta);

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
    };

    Ok(book_meta)
}
