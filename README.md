# raise

Run or raise implemented for niri. It will raise window if it exists,
or cycle to next window if current window matches app id to focus. Otherwise
it will launch new window.

```
$ raise
Usage: raise -c <class> -e <launch>

Raise window if it exists, otherwise launch new window.

Options:
  -c, --class       app id to focus
  -e, --launch      command to launch
  --help            display usage information
```

## Install `raise`

There are multiple ways to install this:

1. `cargo install --git https://github.com/svelterust/raise --branch niri`
2. Add `github:svelterust/raise/niri` as a flake to your NixOS configuration

For NixOS, add raise to your flake inputs:

```nix
inputs = {
  raise.url = "github:svelterust/raise/niri";
};
```

Then add it to your system, for instance: `environment.systemPackages = [raise.defaultPackage.x86_64-linux];`

## Example configuration

I like having <kbd>Super</kbd> + `<key>` bound to run or raise, and <kbd>Super</kbd> + <kbd>Shift</kbd> + `<key>` to launch application regularly.

```kdl
binds {
    Mod+V { spawn "raise" "--class" "Alacritty" "--launch" "alacritty"; }
    Mod+Shift+V { spawn "alacritty"; }
    Mod+C { spawn "raise" "--class" "firefox" "--launch" "firefox"; }
    Mod+Shift+C { spawn "firefox"; }
    Mod+F { spawn "raise" "--class" "emacs" "--launch" "emacsclient --create-frame"; }
    Mod+Shift+F { spawn "emacsclient" "--create-frame"; }
}
```

## How to find app id?

Run `niri msg windows` while window is open, and look for the `app_id` field.
