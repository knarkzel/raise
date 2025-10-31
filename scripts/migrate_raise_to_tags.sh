#!/usr/bin/env bash
set -euo pipefail

# migrate_raise_to_tags.sh
#
# Migrates a Hyprland config repo to:
# 1) Use your fork of raise (latest, unpinned) in flake.nix
# 2) Add window rules assigning tags to common apps
# 3) Replace raise usages to prefer `--tag` instead of class-based matching
# 4) Create commits with the changes
#
# Usage:
#   scripts/migrate_raise_to_tags.sh -r /path/to/hypr-config -u github:<fork>/raise [--no-commit]
#
# Example:
#   scripts/migrate_raise_to_tags.sh -r "$HOME/src/dotfiles" -u github:neg-serg/raise
#

REPO=""
FORK_URL_RAW=""
DO_COMMIT=1

die() { echo "Error: $*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -r|--repo)
      REPO="$2"; shift 2;;
    -u|--fork-url)
      FORK_URL_RAW="$2"; shift 2;;
    --no-commit)
      DO_COMMIT=0; shift;;
    -h|--help)
      sed -n '1,60p' "$0"; exit 0;;
    *)
      die "Unknown argument: $1";;
  esac
done

[[ -n "$REPO" ]] || die "--repo is required"
[[ -n "$FORK_URL_RAW" ]] || die "--fork-url is required (e.g. github:neg-serg/raise or https://github.com/neg-serg/raise)"

[[ -d "$REPO" ]] || die "Repo path not found: $REPO"

if [[ ! -d "$REPO/.git" ]]; then
  die "Target path is not a git repo: $REPO"
fi

pushd "$REPO" >/dev/null

# Normalize fork url to flake-friendly form if plain https was provided
FORK_URL="$FORK_URL_RAW"
if [[ "$FORK_URL_RAW" =~ ^https://github.com/([^/]+)/([^/]+?)(\.git)?$ ]]; then
  FORK_URL="github:${BASH_REMATCH[1]}/${BASH_REMATCH[2]}"
fi

# Create branch if not on a feature branch
current_branch=$(git rev-parse --abbrev-ref HEAD)
if [[ "$current_branch" == "HEAD" ]]; then
  git checkout -b feat/raise-tags-migration || true
elif [[ "$current_branch" == "master" || "$current_branch" == "main" ]]; then
  git checkout -b feat/raise-tags-migration || true
fi

# 1) Point flake input to user's fork and remove explicit rev pin if present
if [[ -f flake.nix ]]; then
  # Replace inputs.raise.url to provided fork URL; add it if missing
  if rg -n "inputs\.raise\.url" -S >/dev/null 2>&1; then
    # Replace the entire value conservatively without grouping to avoid shell parsing quirks
    sed -i -E "s|inputs\.raise\.url\s*=\s*\"[^\"]*\";|inputs.raise.url = \"${FORK_URL}\";|" flake.nix || true
  else
    # Try to inject under inputs = { ... } block
    awk -v url="$FORK_URL" '
      BEGIN{in_inputs=0}
      /inputs\s*=\s*\{/ {in_inputs=1}
      {print}
      in_inputs && /\{/ && !done { print "    raise.url = \"" url "\";"; done=1 }
      in_inputs && /\}/ {in_inputs=0}
    ' flake.nix > flake.nix.tmp && mv flake.nix.tmp flake.nix
  fi

  # Remove explicit pins in flake.nix if present (rev/ref)
  sed -i -E "/inputs\.raise\.(rev|ref)\s*=\s*\".*\";$/d" flake.nix || true

  # Suggest updating lock file; we only stage flake.nix here.
else
  echo "Note: flake.nix not found; skipping fork update." >&2
fi

# 2) Ensure a tags rules file exists with sensible defaults (support multiple hypr roots)
# Discover hypr roots
mapfile -t HYPR_ROOTS < <(find . -type d -path "*/modules/user/gui/hypr" 2>/dev/null | sed 's#^\./##')
if [[ ${#HYPR_ROOTS[@]} -eq 0 ]]; then
  mapfile -t HYPR_ROOTS < <(find . -type d -path "*/nix/.config/home-manager/modules/user/gui/hypr" 2>/dev/null | sed 's#^\./##')
fi
if [[ ${#HYPR_ROOTS[@]} -eq 0 ]]; then
  echo "Warning: Could not locate modules/user/gui/hypr in repo; continuing with flake pin update only." >&2
fi

for HYPR_ROOT in "${HYPR_ROOTS[@]}"; do
  RULES_DIR="${HYPR_ROOT}/conf/rules"
  RULES_FILE="$RULES_DIR/tags.conf"
  mkdir -p "$RULES_DIR"
  cat > "$RULES_FILE" <<'RULES'
# Auto-assigned tags for common applications
# Edit to taste. Each rule applies a tag based on window class.

# Browsers
windowrulev2 = tag, web, class:^(firefox|Firefox|floorp|Floorp|Brave.*|Chromium|Google-chrome|Vivaldi.*)$

# Terminals
windowrulev2 = tag, term, class:^(Alacritty|alacritty|kitty|Kitty|WezTerm|wezterm|foot)$

# Editors / IDEs
windowrulev2 = tag, code, class:^(code|Code|codium|VSCodium|jetbrains-.*|Idea|CLion|PyCharm|GoLand|WebStorm|Rider)$

# Chat / IM
windowrulev2 = tag, chat, class:^(Slack|slack|Discord|discord|TelegramDesktop|telegram-desktop|Element.*)$

# Files
windowrulev2 = tag, files, class:^(thunar|Thunar|dolphin|Dolphin|nemo|Nemo|nautilus|pcmanfm)$

# Media / Video
windowrulev2 = tag, vid, class:^(mpv|vlc|celluloid|Celluloid)$

# Image viewers
windowrulev2 = tag, img, class:^(swayimg|imv|feh)$

# Music
windowrulev2 = tag, music, class:^(spotify|Spotify|ncspot)$

# Notes / Knowledge
windowrulev2 = tag, notes, class:^(obsidian|Obsidian|logseq|Logseq|Zettlr)$

# Mail
windowrulev2 = tag, mail, class:^(thunderbird|Thunderbird)$

# Graphics
windowrulev2 = tag, design, class:^(gimp|Gimp|inkscape|Inkscape|krita|Krita)$

# Audio tools
windowrulev2 = tag, audio, class:^(qpwgraph|Carla2|REAPER)$

# Reading / Documents
windowrulev2 = tag, read, class:^(org\.pwmt\.zathura|zathura)$

# Gaming
windowrulev2 = tag, games, class:^(steam|lutris)$
RULES
done

# 3) Replace raise usages to prefer --tag where mapping is obvious
# Known mapping pairs for common apps (class -> tag)
map_class_to_tag() {
  case "$1" in
    firefox|Firefox|floorp|Floorp|Brave*|Chromium|Google-chrome|Vivaldi*) echo web ;;
    Alacritty|alacritty|kitty|Kitty|WezTerm|wezterm|foot) echo term ;;
    code|Code|codium|VSCodium|jetbrains-*|Idea|CLion|PyCharm|GoLand|WebStorm|Rider) echo code ;;
    Slack|slack|Discord|discord|TelegramDesktop|telegram-desktop|Element*) echo chat ;;
    thunar|Thunar|dolphin|Dolphin|nemo|Nemo|nautilus|pcmanfm) echo files ;;
    mpv|vlc|celluloid|Celluloid) echo vid ;;
    swayimg|imv|feh) echo img ;;
    spotify|Spotify|ncspot) echo music ;;
    obsidian|Obsidian|logseq|Logseq|Zettlr) echo notes ;;
    thunderbird|Thunderbird) echo mail ;;
    gimp|Gimp|inkscape|Inkscape|krita|Krita) echo design ;;
    steam|lutris) echo games ;;
    qpwgraph|Carla2|REAPER) echo audio ;;
    org.pwmt.zathura|zathura) echo read ;;
    term) echo term ;;
    nwim) echo nwim ;;
    *) return 1 ;;
  esac
}

changed_files=()

# Update specific known bindings file if present
for HYPR_ROOT in "${HYPR_ROOTS[@]}"; do
  file="${HYPR_ROOT}/conf/bindings/apps.conf"
  [[ -f "$file" ]] || continue
  tmp=$(mktemp)
  while IFS= read -r line; do
    if [[ "$line" =~ raise[[:space:]].*--class[[:space:]]\"([^\"]+)\" ]]; then
      klass=${BASH_REMATCH[1]}
      if tag=$(map_class_to_tag "$klass"); then
        line=$(echo "$line" | sed -E "s/--class \"[^\"]+\"/--tag ${tag}/g")
      fi
    fi
    # Convert explicit matchers like --match class=Foo to --tag where possible
    if [[ "$line" =~ --match[[:space:]]class=([^[:space:]]+) ]]; then
      klass=${BASH_REMATCH[1]}
      if tag=$(map_class_to_tag "$klass"); then
        line=$(echo "$line" | sed -E "s/--match class=[^[:space:]]+/--tag ${tag}/g")
      fi
    fi
    echo "$line" >> "$tmp"
  done < "$file"
  if ! cmp -s "$file" "$tmp"; then
    mv "$tmp" "$file"
    changed_files+=("$file")
  else
    rm -f "$tmp"
  fi
done

# Broad pass across hypr conf tree: best-effort replacements
for HYPR_ROOT in "${HYPR_ROOTS[@]}"; do
  while IFS= read -r -d '' f; do
    tmp=$(mktemp)
    modified=0
    while IFS= read -r line; do
      if [[ "$line" =~ raise[[:space:]].*--class[[:space:]]\"([^\"]+)\" ]]; then
        klass=${BASH_REMATCH[1]}
        if tag=$(map_class_to_tag "$klass"); then
          line=$(echo "$line" | sed -E "s/--class \"[^\"]+\"/--tag ${tag}/g")
          modified=1
        fi
      fi
      if [[ "$line" =~ --match[[:space:]]class=([^[:space:]]+) ]]; then
        klass=${BASH_REMATCH[1]}
        if tag=$(map_class_to_tag "$klass"); then
          line=$(echo "$line" | sed -E "s/--match class=[^[:space:]]+/--tag ${tag}/g")
          modified=1
        fi
      fi
      echo "$line" >> "$tmp"
    done < "$f"
    if [[ $modified -eq 1 ]]; then
      mv "$tmp" "$f"
      changed_files+=("$f")
    else
      rm -f "$tmp"
    fi
  done < <(find "$HYPR_ROOT" -type f -name '*.conf' -print0 2>/dev/null)
done

# Add a canonical example: bind for web tag with $browser
for HYPR_ROOT in "${HYPR_ROOTS[@]}"; do
  file="${HYPR_ROOT}/conf/bindings/apps.conf"
  [[ -f "$file" ]] || continue
  if ! rg -n "raise --tag web" "$file" >/dev/null 2>&1; then
    echo "bind = \$M4, w, exec, raise --tag web --launch \$browser" >> "$file"
    changed_files+=("$file")
  fi
done

# 3b) Update Home Manager prewarm execs to use --tag where possible
if [[ -f nix/.config/home-manager/home.nix ]]; then
  sed -i -E \
    -e "s#raise --class 'term'#raise --tag term#g" \
    -e "s#raise --class '\(one\\.ablaze\\.floorp\|floorp\)'#raise --tag web#g" \
    -e "s#raise --class 'org\.nicotine_plus\.Nicotine'#raise --tag music#g" \
    -e "s#raise --class 'Obsidian'#raise --tag notes#g" \
    nix/.config/home-manager/home.nix || true
  changed_files+=("nix/.config/home-manager/home.nix")
fi

# 3c) Stop shadowing system raise: rename local script to raise_class if present
if [[ -f nix/.config/home-manager/modules/user/local-bin/default.nix ]]; then
  sed -i -E "s#name = \"raise\";#name = \"raise_class\";#" nix/.config/home-manager/modules/user/local-bin/default.nix || true
  changed_files+=("nix/.config/home-manager/modules/user/local-bin/default.nix")
fi

# Commit staged changes
if [[ $DO_COMMIT -eq 1 ]]; then
  git add -A
  if ! git diff --cached --quiet; then
    git commit -m "hypr: migrate raise usage to tags and add window tag rules

- Switch raise calls to use --tag where mapping is known
- Add rules to auto-assign tags by class (web, term, code, chat, etc.)
- Point flake input 'raise' to ${FORK_URL} (update lock separately)"
  fi
fi

# Try to update lock for raise input if nix is available
if command -v nix >/dev/null 2>&1 && [[ -f flake.nix ]]; then
  nix flake update --update-input raise || true
fi

echo "Migration finished. Next steps:" >&2
echo "- If using flakes, run: nix flake update --update-input raise" >&2
echo "- Reload Hyprland: hyprctl reload" >&2
echo "- Test: raise --tag web --launch \$browser" >&2

# Report any remaining class-based usages for manual follow-up
echo "\nRemaining occurrences of class-based raise (review manually):" >&2
rg -n "raise.*(--class|--match[[:space:]]class=)" -S || true

# Hint to ensure rules are sourced
if [[ ${#HYPR_ROOTS[@]} -gt 0 ]]; then
  echo "\nEnsure your Hypr config sources the tags rules (if not already):" >&2
  for HYPR_ROOT in "${HYPR_ROOTS[@]}"; do
    echo "  source = ~/.config/hypr/conf/rules/tags.conf (root: $HYPR_ROOT)" >&2
  done
fi

popd >/dev/null
