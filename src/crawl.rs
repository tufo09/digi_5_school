use serde::{Deserialize, Serialize};

use crate::util::ApiClient;

// const R_0: &str = r#""#;

/// Pattern for parsing ebook details from the ebooks page.
///
/// The pattern is as follows:
///
/// 1. The relative URL of the ebook
/// 2. The code of the ebook
/// 3. The ID of the ebook
/// 4. The visibility of the ebook
/// 5. The URL of the cover image of the ebook
/// 6. The title of the ebook
/// 7. The publisher of the ebook
/// 8. The expiry date of the ebook with padding
///
const BOOK_REGEX: &str = r#"<a href='([^']+)'[^>]+?data-code='([^']+)' data-id='([^']+)' class='(bag|all)'[^>]*>.+?src='([^']+)'.+?<h1>([^']+)</h1>.+?<span class='publisher'>([^<]+)</span>.+?<h4>([^<]+)</h4>.+?</a>"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBook {
    pub url: String,
    pub code: String,
    pub id: String,
    pub visibility: String,
    pub cover_url: String,
    pub title: String,
    pub publisher: String,
    pub expiry_date: String,
}

pub async fn get_books(
    ApiClient(client, cookie_store): ApiClient,
) -> anyhow::Result<Vec<ParsedBook>> {
    // HACK move this regex to a static variable
    let regex = regex::Regex::new(BOOK_REGEX).unwrap();

    let response = client.get("https://digi4school.at/ebooks").send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Got non-success status code: {}",
            response.status()
        ));
    };

    let text = response.text().await?;

    let mut books = Vec::new();

    for capture in regex.captures_iter(&text) {
        let book = ParsedBook {
            url: capture[1].to_string(),
            code: capture[2].to_string(),
            id: capture[3].to_string(),
            visibility: capture[4].to_string(),
            cover_url: capture[5].to_string(),
            title: capture[6].to_string(),
            publisher: capture[7].to_string(),
            expiry_date: capture[8].to_string(),
        };

        books.push(book);
    }

    Ok(books)
}
