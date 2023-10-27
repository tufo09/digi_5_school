use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::util::ApiClient;

lazy_static! {
    static ref BOOK_HTML_TITLE_REGEX: Regex = Regex::new(r#"action='([^']+)'"#).unwrap();
    static ref BOOK_HTML_FORM_REGEX: Regex =
        Regex::new(r#"<input name='([^']+)' value='([^']*)'>"#).unwrap();
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
