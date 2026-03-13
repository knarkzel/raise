use anyhow::{bail, Context, Result};
use argh::FromArgs;
use serde::Deserialize;
use std::process::{Child, Command};

#[derive(FromArgs)]
/// Raise window if it exists, otherwise launch new window.
struct Args {
    /// app id to focus
    #[argh(option, short = 'c')]
    class: String,

    /// command to launch
    #[argh(option, short = 'e')]
    launch: String,
}

#[derive(Deserialize, Debug)]
struct Client {
    app_id: Option<String>,
    id: u64,
}

fn launch_command(args: &Args) -> std::io::Result<Child> {
    Command::new("niri")
        .args(["msg", "action", "spawn", "--", &args.launch])
        .spawn()
}

fn focus_window(id: u64) -> std::io::Result<Child> {
    Command::new("niri")
        .args(["msg", "action", "focus-window", "--id", &id.to_string()])
        .spawn()
}

fn get_current_matching_window(class: &str) -> Result<Client> {
    let output = Command::new("niri")
        .args(["msg", "--json", "focused-window"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)
        .context("Reading `niri msg focused-window` to string failed")?;
    let client: Client = serde_json::from_str(&stdout).context("Failed to parse focused window")?;
    if client.app_id.as_deref() == Some(class) {
        Ok(client)
    } else {
        bail!("Current window is not of same class")
    }
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();

    let json = Command::new("niri")
        .args(["msg", "--json", "windows"])
        .output();
    match json {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)
                .context("Reading `niri msg windows` to string failed")?;
            let clients: Vec<Client> =
                serde_json::from_str(&stdout).context("Failed to parse `niri msg windows`")?;

            let candidates = clients
                .iter()
                .filter(|client| client.app_id.as_deref() == Some(&args.class))
                .collect::<Vec<_>>();

            if let Ok(Client { id, .. }) = get_current_matching_window(&args.class) {
                if let Some(index) = candidates.iter().position(|client| client.id == id) {
                    if let Some(next_client) = candidates.iter().cycle().skip(index + 1).next() {
                        focus_window(next_client.id)?;
                    }
                }
            } else {
                match candidates.first() {
                    Some(client) => focus_window(client.id)?,
                    _ => launch_command(&args)?,
                };
            }
        }
        _ => {
            launch_command(&args)?;
        }
    }

    Ok(())
}
