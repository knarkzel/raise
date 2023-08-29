# raise

Run or raise implemented for Hyprland. It will raise window if it exists,
and cycle to next window if current window is of same class. Otherwise
it will launch new window.

```
$ raise
Usage: raise -c <class> -e <launch>

Raise window if it exists, otherwise create new instance.

Options:
  -c, --class       class to focus
  -e, --launch      command to launch
  --help            display usage information
```

## Example configuration

I like having Super + <key> bound to run or raise, and Super + Shift
+ <key> to launch application regularly.

```
bind = SUPER, V, exec, /home/odd/.cargo/bin/raise --class "Alacritty" --launch "alacritty"
bind = SUPER_SHIFT, V, exec, alacritty
bind = SUPER, C, exec, /home/odd/.cargo/bin/raise --class "firefox" --launch "firefox"
bind = SUPER_SHIFT, C, exec, firefox
bind = SUPER, F, exec, /home/odd/.cargo/bin/raise --class "emacs" --launch "emacsclient --create-frame"
bind = SUPER_SHIFT, F, exec, emacsclient --create-frame
```

## How to find class?

Run `hyprcl clients` while window is open, and look for `class: <class>`.
