use anyhow::{Context, Result, bail};
use argh::FromArgs;
use miniserde::{json, Deserialize};
use std::process::{Child, Command};

#[derive(FromArgs)]
/// Raise window if it exists, otherwise launch new window.
struct Args {
    /// class to focus
    #[argh(option, short = 'c')]
    class: String,

    /// command to launch
    #[argh(option, short = 'e')]
    launch: String,
}

#[derive(Deserialize, Debug)]
struct Client {
    class: String,
    address: String,
}

fn launch_command(args: &Args) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("keyword")
        .arg("exec")
        .args(args.launch.split_whitespace())
        .spawn()
}

fn focus_window(address: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg("focuswindow")
        .arg(format!("address:{address}"))
        .spawn()
}

fn get_current_matching_window(class: &str) -> Result<Client> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()?;
    let stdout = String::from_utf8(output.stdout)
        .context("Reading `hyprctl currentwindow -j` to string failed")?;
    let client = json::from_str::<Client>(&stdout)?;
    if class == &client.class {
        Ok(client)
    } else {
        bail!("Current window is not of same class")
    }
}

fn main() -> Result<()> {
    // Get arguments
    let args: Args = argh::from_env();

    // Launch hyprctl
    let json = Command::new("hyprctl").arg("clients").arg("-j").output();
    match json {
        Ok(output) if output.status.success() => {
            // Deserialize output
            let stdout = String::from_utf8(output.stdout)
                .context("Reading `hyprctl clients -j` to string failed")?;
            let clients = json::from_str::<Vec<Client>>(&stdout)
                .context("Failed to parse `hyprctl clients -j`")?;

            // Filter matching clients
            let candidates = clients
                .iter()
                .filter(|client| client.class == args.class)
                .collect::<Vec<_>>();
            
            // Are we currently focusing a window of this class?
            if let Ok(Client { address, .. }) = get_current_matching_window(&args.class) {
                // Focus next window based on first
                if let Some(index) = candidates.iter().position(|client| client.address == address) {
                    if let Some(next_client) = candidates.iter().cycle().skip(index + 1).next() {
                        focus_window(&next_client.address)?;
                    }
                }
            } else {
                // Focus first window, otherwise launch command
                match candidates.first() {
                    Some(Client { address, .. }) => focus_window(address)?,
                    _ => launch_command(&args)?,
                };
            }
        }
        // If hyprctl fails, just launch it
        _ => {
            launch_command(&args)?;
        }
    }

    // Success
    Ok(())
}
