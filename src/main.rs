use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;
use crossterm::style::Stylize;
use tokio::sync::mpsc;

pub mod session;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    identity_file: Option<PathBuf>,
    #[arg(short, long)]
    servers: Vec<String>,
    #[arg(short, long)]
    files: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let servers = cli
        .servers
        .iter()
        .map(|server| server.split(',').collect::<Vec<&str>>())
        .flatten()
        .filter_map(|s| s.split_once('@'))
        .collect::<Vec<(&str, &str)>>();

    let (tx, mut rx) = mpsc::unbounded_channel();

    for (user, host) in servers {
        let identity_file = check_identity_file(cli.identity_file.clone())?;
        let files = cli
            .files
            .clone()
            .iter()
            .map(|f| f.to_str().unwrap())
            .collect::<Vec<&str>>()
            .as_slice()
            .join(" ");

        let tx = tx.clone();
        let (user, host) = (user.to_string(), host.to_string());

        tokio::spawn(async move {
            match session::Session::connect(format!("{user}@{host}"), identity_file, user, &host)
                .await
            {
                Ok(mut session) => {
                    session.call(&format!("tail -f {}", files), tx).await;
                }
                Err(err) => eprintln!("Failed to connect to {}: {}", host, err),
            }
        });
    }

    loop {
        if let Some((id, data)) = rx.recv().await {
            print!("[{}] --> {}", id.green(), String::from_utf8_lossy(&data));
        }
    }

    Ok(())
}

const DEFAULT_IDENTITY_FILES: [&'static str; 4] = [
    "~/.ssh/id_rsa",
    "~/.ssh/id_dsa",
    "~/.ssh/id_ecdsa",
    "~/.ssh/id_ed25519",
];

fn check_identity_file(identity_file: Option<PathBuf>) -> Result<PathBuf> {
    if identity_file.is_none() {
        for file in DEFAULT_IDENTITY_FILES.iter() {
            let file = PathBuf::from(file);
            if file.exists() {
                return Ok(file);
            }
        }
        color_eyre::eyre::bail!("No identity file found");
    }

    Ok(identity_file.unwrap())
}
