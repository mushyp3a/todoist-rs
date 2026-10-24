use std::{
    fs,
    io::stdin,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};

use crate::banner;

#[derive(Deserialize, Serialize)]
struct Config {
    bearer_token: String,
}

fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| eyre!("No config directory found"))?
        .join(".todoist");
    fs::create_dir_all(&dir).wrap_err("Failed to create the config directory")?;
    Ok(dir.join("config.json"))
}

/// Returns the saved API token, asking for it on first run.
pub fn load_token() -> Result<String> {
    let path = config_path()?;
    if path.exists() {
        let file = fs::File::open(&path).wrap_err("Failed to open the config file")?;
        let config: Config = serde_json::from_reader(file)
            .wrap_err_with(|| format!("Invalid config file {}", path.display()))?;
        Ok(config.bearer_token)
    } else {
        prompt_for_token(&path)
    }
}

fn prompt_for_token(path: &Path) -> Result<String> {
    println!("{}\n", banner::BANNER);
    println!("\nHow to get setup:");
    println!("-----------------\n");

    let instructions = [
        "Go to the todoist integrations - https://app.todoist.com/app/settings/integrations/developer",
        "Under the developer tab, you will be able to see the API Token",
        "Copy the token and paste it below",
        "You are now ready to authenticate with Todoist!",
        &format!("Config will be saved to {}\n\n", path.display()),
    ];
    for (number, item) in instructions.iter().enumerate() {
        println!("  {}. {}", number + 1, item);
    }

    println!("\nEnter your API token:");
    let mut token = String::new();
    stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    let config = Config {
        bearer_token: token.clone(),
    };
    fs::write(path, serde_json::to_string(&config)?).wrap_err("Failed to save the config file")?;
    Ok(token)
}
