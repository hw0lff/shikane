# shikane
A dynamic output configuration tool that automatically detects and configures
connected outputs based on a set of profiles.

Each profile specifies a set of outputs with additional parameters (e.g., mode).
A profile will be applied automatically if all specified outputs and modes can
be perfectly matched to the currently connected outputs and their capabilities.

This is a Wayland equivalent for tools like [autorandr].
It aims to fully replace [kanshi] and add new features.
shikane works with Wayland compositors supporting version 3 of the
wlr-output-management protocol (e.g., compositors using wlroots v0.16).

## Features
| feature | kanshi | shikane |
| ------- | ------ | ------- |
| configure output properties: resolution, refresh rate, position, scaling, transformation | yes | yes |
| execute commands | yes | yes |
| output names are supplied to executed commands | no | yes |
| full cardinality matching algorithm | no | yes |
| regex based output matching | no | yes |
| state machine defined execution | no | yes |
| one-shot mode | no | yes |

## Installation
Via cargo from [crates.io]:
```sh
cargo install shikane
```

Via your `$AURhelper` from the [AUR]:
```sh
$AURhelper -S shikane
```

## Documentation
Documentation is provided as man pages:
```sh
man 1 shikane
man 5 shikane
```

## Usage
1. Create your configuration file.
    See [configuration](#configuration) for a short overview
    or have a look at the man page `man 5 shikane` for more detailed information.
2. Start shikane.
    ```sh
    shikane
    ```

## Building
Dependencies:
- a rust toolchain >=1.60
- pandoc (for building the man pages)

Building shikane:
```sh
cargo build --release
```

Building the man pages:
```sh
./scripts/build-docs.sh man

man -l build/shikane.1.gz
man -l build/shikane.5.gz
```

## Configuration
shikane uses the [TOML] file format for its configuration file
and contains an array of **profile**s. Each **profile** is a table containing an
array of **output** tables. The configuration file should be placed at
`$XDG_CONFIG_HOME/shikane/config.toml`.

```toml
[[profile]]
name = "dual_foo"
exec = ["notify-send shikane \"Profile $SHIKANE_PROFILE_NAME has been applied\""]
    [[profile.output]]
    match = "Company Foo"
    enable = true
    mode = { width = 1920, height = 1080, refresh = 50 }
    position = { x = 0, y = 0 }
    scale = 1.3

    [[profile.output]]
    match = "/HDMI-[ABC]-[1-9]/"
    enable = true
    exec = ["echo This is output $SHIKANE_OUTPUT_NAME"]
    mode = { width = 2560, height = 1440, refresh = 75 }
    position = { x = 1920, y = 0 }
    transform = "270"
```

## Acknowledgements
- [kanshi] being the inspiration and motivation
- [wayland-rs] providing the wayland bindings

## License
MIT


[AUR]: https://aur.archlinux.org/packages/shikane
[autorandr]: https://github.com/phillipberndt/autorandr
[crates.io]: https://crates.io/crates/shikane
[kanshi]: https://sr.ht/~emersion/kanshi
[TOML]: https://toml.io
[wayland-rs]: https://github.com/Smithay/wayland-rs
