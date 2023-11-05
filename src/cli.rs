use inquire::{Password, Text};

use crate::login::Credentials;

pub fn get_credentials() -> anyhow::Result<Credentials> {
    let email = Text::new("Email:").prompt()?;
    let password = Password::new("Password:").without_confirmation().prompt()?;

    Ok(Credentials { email, password })
}
