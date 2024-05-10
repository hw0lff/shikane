# shikane
A dynamic output configuration tool focusing on accuracy and determinism.

It automatically detects and configures connected displays based on a set of
profiles. Each profile specifies a set of outputs with additional parameters
(e.g., mode, position, scale). A profile will be applied automatically if all
specified outputs and modes can be *perfectly* matched to the currently
connected displays and their capabilities.

This is a Wayland equivalent for tools like [autorandr].
It aims to fully replace [kanshi], surpass its inaccuracies and add new features.
shikane works with Wayland compositors supporting versions >=3 of the
wlr-output-management protocol (e.g., compositors using wlroots v0.16).

## Features
- generation of *all* compatible (display, output, mode)-combinations, ranked by exactness
- specify multiple matching rules per output
- restrict the matching to only certain display attributes
- choose between regex, substring or full text based attribute matching
- full cardinality matching algorithm
- ad-hoc profile switching
- export current display setup as shikane config.toml
- state machine defined execution
- execute commands, profile and display names are supplied as env vars
- one-shot mode

## How the matching process works
shikane selects possible **profile**s automatically at startup and when a change
in the set of currently connected displays occurs.
A **profile** is taken into consideration if every currently connected display
can be matched to at least one **output** and no **output** is unmatched.\
A display matches an **output** if:

- the **search** parameter matches against the properties of the display
- AND the display supports the **mode** that is specified in the **output**
  table.

After assembling a list of possible **profile**s shikane generates all variants
of every **profile**. Once all variants have been verified and sorted by
exactness, shikane tries to apply them one after the other until one succeeds
or there are no variants left to try.

Variants are slightly different versions of the same **profile**.\
For example, a given display has a set of supported modes: 1920x1080@60Hz and
1920x1080@50Hz. If the **mode** in the config.toml is specified as "1920x1080"
both modes would fit the specification. Instead of choosing just one mode and
using that, shikane takes both into account by generating two variants based on
the same **profile**. One variant uses the 1920x1080@60Hz mode and the other
variant uses the 1920x1080@50Hz mode.\
The same goes for the search parameter. If multiple
(display,**output**,mode)-combinations are possible, shikane generates variants with
all of them.

## Usage
1. Create your configuration file.
    See [configuration](#configuration) for a short overview
    or have a look at the man page `man 5 shikane` for more detailed information.
2. Start shikane.
    ```sh
    shikane
    ```

### Using `shikanectl` to generate configurations
1. Start shikane.
2. Configure your outputs by hand with a (GUI) tool.
3. Export the current configuration.
    ```sh
    shikanectl export "room04"
    ```
4. Append the printed [TOML] to shikanes config.toml.
5. Reload the config file.
    ```sh
    shikanectl reload
    ```

## Installation
Via your `$AURhelper` from the [AUR]:
```sh
$AURhelper -S shikane
```

Via cargo from [crates.io] (without man pages):
```sh
cargo install shikane
```

## Documentation
Documentation is provided as man pages:
```sh
man 1 shikane
man 5 shikane
man 1 shikanectl
```

## Building
Dependencies:
- a rust toolchain >=1.70
- pandoc (for building the man pages)

Building shikane:
```sh
cargo build --release
```

Building the man pages:
```sh
./scripts/build-docs.sh man

man -l build/man/shikane.1.gz
man -l build/man/shikane.5.gz
man -l build/man/shikanectl.1.gz
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
    # search for a matching serial number and model by full text comparison
    search = ["s=SERIAL123", "m=1Q2X5T"]
    enable = true
    mode = "1920x1080@50"
    position = "0,0"
    scale = 1.3

    [[profile.output]]
    search  = "n/HDMI-[ABC]-[1-9]" # search for a matching name by regex
    enable = true
    exec = ["echo This is output $SHIKANE_OUTPUT_NAME"]
    mode = "2560x1440@75Hz"
    position = "1920,0"
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
