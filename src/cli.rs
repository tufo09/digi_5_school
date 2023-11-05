use inquire::{MultiSelect, Password, Text};

use crate::{crawl::ParsedBook, login::Credentials};

pub fn get_credentials() -> anyhow::Result<Credentials> {
    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:").without_confirmation().prompt()?;

    Ok(Credentials { email, password })
}

pub fn book_selection(books: &[ParsedBook]) -> anyhow::Result<Vec<ParsedBook>> {
    let selection = MultiSelect::new("Select books to download:", books.to_vec()).prompt()?;

    Ok(selection)
}
