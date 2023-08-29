# raise

Run or raise implemented for Hyprland. It will raise window if it exists,
or cycle to next window if current window matches class to focus. Otherwise
it will launch new window.

```
$ raise
Usage: raise -c <class> -e <launch>

Raise window if it exists, otherwise launch new window.

Options:
  -c, --class       class to focus
  -e, --launch      command to launch
  --help            display usage information
```

## Install `raise`

There are multiple ways to install this:

1. Go to [releases](https://github.com/knarkzel/raise/releases)
2. `cargo install --git https://github.com/knarkzel/raise`
3. Add `github:knarkzel/raise` as a flake to your NixOS configuration

## Example configuration

I like having <kbd>Super</kbd> + `<key>` bound to run or raise, and <kbd>Super</kbd> + <kbd>Shift</kbd> + `<key>` to launch application regularly.

```
bind = SUPER, V, exec, raise --class "Alacritty" --launch "alacritty"
bind = SUPER_SHIFT, V, exec, alacritty
bind = SUPER, C, exec, raise --class "firefox" --launch "firefox"
bind = SUPER_SHIFT, C, exec, firefox
bind = SUPER, F, exec, raise --class "emacs" --launch "emacsclient --create-frame"
bind = SUPER_SHIFT, F, exec, emacsclient --create-frame
```

## How to find class?

Run `hyprctl clients` while window is open, and look for `class: <class>`.
