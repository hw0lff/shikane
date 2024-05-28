# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
regarding documented command line interfaces and configuration files.

## [Unreleased]

### Added

- Set `SHIKANE_LOG_TIME` environment variable to `1` to enable timestamps in logs.
  By default no timestamps are logged.
- Document the `SHIKANE_LOG`, `SHIKANE_LOG_STYLES` and `SHIKANE_LOG_TIME` env vars.

### Changed

- Change log filter to print state machine changes by default

### Fixed

- daemon in oneshot mode not shutting down on NoVariantApplied state

## [1.0.0] - 2024-05-27

### Added

#### CLI client `shikanectl` for controlling and querying the shikane daemon

- `shikanectl export` export current display setup as shikane config
- `shikanectl reload` instruct the daemon to reread the config file
- `shikanectl switch` ad-hoc profile switching

#### shikane daemon

- Overhauled, more complex matching procedure which enables the generation of
  all possible profile variants based on connected displays and sorting those
  variants by exactness.
- config: Add `search` field to replace the `match` field
  - Compare `search` *patterns* against specific display attributes
  - Define multiple `search`es per output
  - New search kind *substring matching* in `search`es
- config: Add parsing of `position`s in the form `x,y`
- config: Add parsing of `mode`s like `1920x1080@60Hz`
- Selecting and setting of `best` and `preferred` modes
- Optional `timeout` to wait after certain events from the compositor

#### Meta

- Use the [kanshi-converter snippet](https://gitlab.com/w0lff/shikane/-/snippets/3713247)
  to convert kanshi config to shikanes config.toml.
- Testing for parts of the config parser (e.g. `search`, `mode`)
- flake.nix for building shikane and documentation with [nix](https://nixos.org/)
- [HTML documentation](https://w0lff.gitlab.io/shikane)
- Automated deployment of HTML documentation using .gitlab-ci.yml

### Changed

- Regex comparison behavior.
  Previously, display attributes were concatenated to a single string before
  being checked against a regex. Now, display attributes are independently
  compared with the regex. Refer to the
  [documentation](https://w0lff.gitlab.io/shikane/shikane.5.html) of the
  `search` field for usage instructions.
- Parsing of regexes in config.toml.
  Previously ignored trailing slashes will now be seen as part of the regex.
  See below on how to migrate to the new `search` field syntax.
- Update documentation to describe variants, `search`es and `mode` parsing.
- Default log level to `warn`
- Accept an empty config.toml file
- Update MSRV to 1.70

### Deprecated

- config: Defining a `position` as a table `{ x = 0, y = 0 }`.
  Use a string instead `"x,y"`.

### Removed

- config: The `match` field has been replaced by the `search` field.

#### Migration from [0.2.0]

Use the commands below in order to migrate from the `match` field syntax
to the `search` field syntax.

**Note:** Due to the mentioned changes in how display attributes are supplied
to regexes, your regexes might not work even after removing the trailing slashes.

```shell
# First, remove the trailing slash from regexes in the toml file.
sed -r -i 's#match.*=.*"/(.*)/"#match = "/\1"#' path/to/shikane/config.toml
# Second, rename all match fields to search fields.
sed -r -i 's#match.*=.*"(.*)"#search = "\1"#' path/to/shikane/config.toml
```

## [0.2.0] - 2023-04-30

### Added

- Support for `adaptive_sync` option

## [0.1.2] - 2023-04-29

### Fixed

- docs: Missing version increment

## [0.1.1] - 2023-04-29

### Added

- docs: Add acknowledgements, usage and feature comparison

## [0.1.0] - 2023-02-10

### Added

- shikane daemon
- documentation in man pages
- MIT License


[1.0.0]: https://gitlab.com/w0lff/shikane/-/compare/v0.2.0...v1.0.0
[0.2.0]: https://gitlab.com/w0lff/shikane/-/compare/v0.1.2...v0.2.0
[0.1.2]: https://gitlab.com/w0lff/shikane/-/compare/v0.1.1...v0.1.2
[0.1.1]: https://gitlab.com/w0lff/shikane/-/compare/v0.1.0...v0.1.1
[0.1.0]: https://gitlab.com/w0lff/shikane/releases/tag/v0.1.0
