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

enum ConfigType {
    Hyprlang,
    Lua,
}

fn config_type() -> ConfigType {
    let output = Command::new("hyprctl")
        .arg("eval")
        .arg("print('lua')")
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let text = format!("{stdout}{stderr}");
        if text.contains("eval is only supported with the lua config manager") {
            return ConfigType::Hyprlang;
        }
        if output.status.success() {
            return ConfigType::Lua;
        }
    }

    ConfigType::Hyprlang
}

fn lua_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn launch_command(args: &Args, config_type: &ConfigType) -> std::io::Result<Child> {
    let mut command = Command::new("hyprctl");
    match config_type {
        ConfigType::Hyprlang => command.arg("keyword").arg("exec").arg(&args.launch),
        ConfigType::Lua => command
            .arg("dispatch")
            .arg(format!("hl.dsp.exec_cmd({})", lua_string(&args.launch))),
    }
    .spawn()
}

fn focus_window(address: &str, config_type: &ConfigType) -> std::io::Result<Child> {
    let mut command = Command::new("hyprctl");
    match config_type {
        ConfigType::Hyprlang => command
            .arg("dispatch")
            .arg("focuswindow")
            .arg(format!("address:{address}")),
        ConfigType::Lua => command.arg("dispatch").arg(format!(
            "hl.dsp.focus({{ window = {} }})",
            lua_string(&format!("address:{address}"))
        )),
    }
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
    let config_type = config_type();

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
                        focus_window(&next_client.address, &config_type)?;
                    }
                }
            } else {
                // Focus first window, otherwise launch command
                match candidates.first() {
                    Some(Client { address, .. }) => focus_window(address, &config_type)?,
                    _ => launch_command(&args, &config_type)?,
                };
            }
        }
        // If hyprctl fails, just launch it
        _ => {
            launch_command(&args, &config_type)?;
        }
    }

    // Success
    Ok(())
}
