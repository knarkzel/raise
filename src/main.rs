use anyhow::{bail, Context, Result};
use argh::FromArgs;
use miniserde::{json, Deserialize};
use regex::Regex;
use std::process::{Child, Command};

#[derive(Debug, Clone)]
struct MatchCondition {
    field: MatchField,
    matcher: Matcher,
}

impl MatchCondition {
    fn new(field: MatchField, matcher: Matcher) -> Self {
        Self { field, matcher }
    }

    fn matches(&self, client: &Client) -> bool {
        match self.field {
            MatchField::Tag => {
                if let Some(tags) = &client.tags {
                    if tags.iter().any(|t| self.matcher.matches(t)) {
                        return true;
                    }
                }
                if let Some(tag) = &client.tag {
                    return self.matcher.matches(tag);
                }
                false
            }
            _ => self
                .field
                .value(client)
                .map(|value| self.matcher.matches(value))
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum MatchField {
    Class,
    InitialClass,
    Title,
    InitialTitle,
    Tag,
    XdgTag,
}

impl MatchField {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "class" | "c" => Some(Self::Class),
            "initial-class" | "initialClass" => Some(Self::InitialClass),
            "title" => Some(Self::Title),
            "initial-title" | "initialTitle" => Some(Self::InitialTitle),
            "tag" => Some(Self::Tag),
            "xdgtag" | "xdg-tag" | "xdgTag" => Some(Self::XdgTag),
            _ => None,
        }
    }

    fn value<'a>(&self, client: &'a Client) -> Option<&'a str> {
        match self {
            Self::Class => Some(client.class.as_str()),
            Self::InitialClass => client.initial_class.as_deref(),
            Self::Title => client.title.as_deref(),
            Self::InitialTitle => client.initial_title.as_deref(),
            Self::Tag => client.tag.as_deref(),
            Self::XdgTag => client.xdg_tag.as_deref(),
        }
    }
}

#[derive(Debug, Clone)]
enum Matcher {
    Equals(String),
    Contains(String),
    Prefix(String),
    Suffix(String),
    Regex(Regex),
}

impl Matcher {
    fn from_tokens(method: Option<&str>, pattern: &str) -> std::result::Result<Self, String> {
        let method = method.unwrap_or("equals");
        match method {
            "equals" | "eq" => Ok(Self::Equals(pattern.to_owned())),
            "contains" | "substr" => Ok(Self::Contains(pattern.to_owned())),
            "prefix" | "starts-with" | "startswith" => Ok(Self::Prefix(pattern.to_owned())),
            "suffix" | "ends-with" | "endswith" => Ok(Self::Suffix(pattern.to_owned())),
            "regex" | "re" => Regex::new(pattern)
                .map(Self::Regex)
                .map_err(|err| format!("Invalid regex `{pattern}`: {err}")),
            _ => Err(format!("Unsupported match method `{method}`")),
        }
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Equals(pattern) => value == pattern,
            Self::Contains(pattern) => value.contains(pattern),
            Self::Prefix(pattern) => value.starts_with(pattern),
            Self::Suffix(pattern) => value.ends_with(pattern),
            Self::Regex(regex) => regex.is_match(value),
        }
    }
}

fn parse_match_condition(value: &str) -> std::result::Result<MatchCondition, String> {
    let (selector, pattern) = value
        .split_once('=')
        .ok_or_else(|| "Expected matcher in the form field[:method]=pattern".to_string())?;

    if pattern.is_empty() {
        return Err("Matcher pattern cannot be empty".to_string());
    }

    let (field_token, method_token) = match selector.split_once(':') {
        Some((field, method)) => (field, Some(method)),
        None => (selector, None),
    };

    let field = MatchField::parse(field_token)
        .ok_or_else(|| format!("Unsupported match field `{field_token}`"))?;

    let matcher = Matcher::from_tokens(method_token, pattern)?;

    Ok(MatchCondition::new(field, matcher))
}

#[derive(FromArgs)]
/// Raise window if it exists, otherwise launch new window.
struct Args {
    /// class to focus (shorthand for `--match class=...`)
    #[argh(option, short = 'c')]
    class: Option<String>,

    /// window tag to match (repeatable; shorthand for `--match tag=...`)
    #[argh(option, long = "tag")]
    tag: Vec<String>,

    /// XDG surface tag to match (repeatable; shorthand for `--match xdgtag=...`)
    #[argh(option, long = "xdgtag")]
    xdgtag: Vec<String>,

    /// command to launch
    #[argh(option, short = 'e')]
    launch: String,

    /// additional matchers in the form field[:method]=pattern
    #[argh(
        option,
        short = 'm',
        long = "match",
        from_str_fn(parse_match_condition)
    )]
    matches: Vec<MatchCondition>,
}

impl Args {
    fn build_matchers(&self) -> Result<Vec<MatchCondition>> {
        let mut matchers = Vec::new();

        if let Some(class) = &self.class {
            matchers.push(MatchCondition::new(
                MatchField::Class,
                Matcher::Equals(class.clone()),
            ));
        }

        for t in &self.tag {
            matchers.push(MatchCondition::new(
                MatchField::Tag,
                Matcher::Equals(t.clone()),
            ));
        }

        for x in &self.xdgtag {
            matchers.push(MatchCondition::new(
                MatchField::XdgTag,
                Matcher::Equals(x.clone()),
            ));
        }

        matchers.extend(self.matches.clone());

        if matchers.is_empty() {
            bail!("Provide at least one matcher via --class or --match");
        }

        Ok(matchers)
    }
}

#[derive(Deserialize, Debug)]
struct Client {
    class: String,
    address: String,
    #[serde(rename = "initialClass")]
    initial_class: Option<String>,
    title: Option<String>,
    #[serde(rename = "initialTitle")]
    initial_title: Option<String>,
    tag: Option<String>,
    // modern Hyprland: array of tags
    tags: Option<Vec<String>>,
    #[serde(rename = "xdgTag")]
    xdg_tag: Option<String>,
}

fn launch_command(args: &Args) -> std::io::Result<Child> {
    Command::new("sh")
        .arg("-c")
        .arg(&args.launch)
        .spawn()
}

fn focus_window(address: &str) -> std::io::Result<Child> {
    Command::new("hyprctl")
        .arg("dispatch")
        .arg(format!(r#"hl.dsp.focus({{window = "address:{address}"}})"#))
        .spawn()
}

fn get_current_matching_window(matchers: &[MatchCondition]) -> Result<Client> {
    let output = Command::new("hyprctl")
        .arg("activewindow")
        .arg("-j")
        .output()?;
    let stdout = String::from_utf8(output.stdout)
        .context("Reading `hyprctl activewindow -j` to string failed")?;
    let client = json::from_str::<Client>(&stdout)?;
    if matchers.iter().all(|matcher| matcher.matches(&client)) {
        Ok(client)
    } else {
        bail!("Current window does not match provided conditions")
    }
}

fn main() -> Result<()> {
    // Get arguments
    let args: Args = argh::from_env();

    let matchers = args.build_matchers()?;

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
                .filter(|client| matchers.iter().all(|matcher| matcher.matches(*client)))
                .collect::<Vec<_>>();

            // Are we currently focusing a window of this class?
            if let Ok(current_client) = get_current_matching_window(&matchers) {
                // Focus next window based on first
                if let Some(index) = candidates
                    .iter()
                    .position(|client| client.address == current_client.address)
                {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_client(
        class: &str,
        initial_class: Option<&str>,
        title: Option<&str>,
        initial_title: Option<&str>,
        tag: Option<&str>,
        xdg_tag: Option<&str>,
    ) -> Client {
        Client {
            class: class.to_owned(),
            address: "0x123".to_owned(),
            initial_class: initial_class.map(str::to_owned),
            title: title.map(str::to_owned),
            initial_title: initial_title.map(str::to_owned),
            tag: tag.map(str::to_owned),
            tags: tag.map(|t| vec![t.to_owned()]),
            xdg_tag: xdg_tag.map(str::to_owned),
        }
    }

    fn matches(condition: &MatchCondition, client: &Client) -> bool {
        condition.matches(client)
    }

    #[test]
    fn matches_class_field() {
        let client = build_client("Firefox", None, None, None, None, None);
        let condition =
            MatchCondition::new(MatchField::Class, Matcher::Equals("Firefox".to_string()));
        assert!(matches(&condition, &client));

        let failing =
            MatchCondition::new(MatchField::Class, Matcher::Equals("Chromium".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_title_field() {
        let client = build_client("Firefox", None, Some("Docs - Firefox"), None, None, None);
        let condition =
            MatchCondition::new(MatchField::Title, Matcher::Contains("Docs".to_string()));
        assert!(matches(&condition, &client));

        let failing =
            MatchCondition::new(MatchField::Title, Matcher::Contains("Other".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_initial_class_field() {
        let client = build_client("Firefox", Some("firefox"), None, None, None, None);
        let condition = MatchCondition::new(
            MatchField::InitialClass,
            Matcher::Equals("firefox".to_string()),
        );
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(
            MatchField::InitialClass,
            Matcher::Equals("kitty".to_string()),
        );
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_initial_title_field() {
        let client = build_client(
            "Firefox",
            None,
            Some("Docs - Firefox"),
            Some("Welcome"),
            None,
            None,
        );
        let condition = MatchCondition::new(
            MatchField::InitialTitle,
            Matcher::Equals("Welcome".to_string()),
        );
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(
            MatchField::InitialTitle,
            Matcher::Equals("Other".to_string()),
        );
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_tag_field() {
        let client = build_client("Firefox", None, None, None, Some("work"), None);
        let condition = MatchCondition::new(MatchField::Tag, Matcher::Equals("work".to_string()));
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(MatchField::Tag, Matcher::Equals("play".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matches_xdgtag_field() {
        let client = build_client("Firefox", None, None, None, None, Some("browser"));
        let condition =
            MatchCondition::new(MatchField::XdgTag, Matcher::Equals("browser".to_string()));
        assert!(matches(&condition, &client));

        let failing = MatchCondition::new(MatchField::XdgTag, Matcher::Equals("video".to_string()));
        assert!(!matches(&failing, &client));
    }

    #[test]
    fn matcher_variants_behave_as_expected() {
        let client = build_client("Firebox", None, Some("Docs - Firebox"), None, None, None);

        let equals = Matcher::from_tokens(Some("equals"), "Firebox").unwrap();
        assert!(equals.matches(&client.class));

        let contains = Matcher::from_tokens(Some("contains"), "Docs").unwrap();
        assert!(contains.matches(client.title.as_deref().unwrap()));

        let prefix = Matcher::from_tokens(Some("prefix"), "Docs").unwrap();
        assert!(prefix.matches(client.title.as_deref().unwrap()));

        let suffix = Matcher::from_tokens(Some("suffix"), "Firebox").unwrap();
        assert!(suffix.matches(client.title.as_deref().unwrap()));

        let regex = Matcher::from_tokens(Some("regex"), "^Docs.*box$").unwrap();
        assert!(regex.matches(client.title.as_deref().unwrap()));
    }

    #[test]
    fn parse_match_condition_supports_aliases() {
        let initial_class = parse_match_condition("initialClass=kitty").unwrap();
        assert!(matches(
            &initial_class,
            &build_client("kitty", Some("kitty"), None, None, None, None)
        ));

        let initial_title = parse_match_condition("initial-title=Welcome").unwrap();
        assert!(matches(
            &initial_title,
            &build_client("App", None, Some("App - now"), Some("Welcome"), None, None)
        ));

        let xdg_tag = parse_match_condition("xdg-tag=browser").unwrap();
        assert!(matches(
            &xdg_tag,
            &build_client("App", None, None, None, None, Some("browser"))
        ));
    }
}
