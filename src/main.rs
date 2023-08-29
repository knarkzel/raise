use anyhow::{Context, Result};
use argh::FromArgs;
use miniserde::{json, Deserialize};
use std::process::{Child, Command};

#[derive(FromArgs)]
/// Raise window if it exists, otherwise create new instance.
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
    pid: usize,
    class: String,
}

fn launch_command(args: &Args) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("keyword")
        .arg("exec")
        .args(args.launch.split_whitespace())
        .spawn()
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

            
            // Focus client
            if let Some(Client { pid, .. }) = candidates.first() {
                // Use dispatch with pid:<pid>
                Command::new("hyprctl")
                    .arg("dispatch")
                    .arg("focuswindow")
                    .arg(format!("pid:{pid}"))
                    .spawn()?;
            } else {
                // Launch command
                launch_command(&args)?;
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
