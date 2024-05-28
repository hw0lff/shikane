# NAME
shikane - deterministic dynamic output configuration


# SYNOPSIS
**shikane** \
**shikane** \[**-hV**\] \[**-o**\] \[**-c** *file*\] \
**shikane** \[**\--oneshot**\] \[**\--config** *file*\]


# DESCRIPTION
shikane (/ʃiˈkaːnə/) is a dynamic output configuration tool focusing on accuracy and determinism.

It automatically detects and configures connected displays based on a set of
profiles. Each profile specifies a set of outputs with additional parameters
(e.g., mode, position, scale). A profile will be applied automatically if all
specified outputs and modes can be perfectly matched to the currently connected
displays and their capabilities.
(See **shikane**(5) for details.)

This is a Wayland equivalent for tools like autorandr.
It aims to fully replace kanshi, surpass its inaccuracies and add new features.
shikane works with Wayland compositors supporting versions >=3 of the
wlr-output-management protocol (e.g., compositors using wlroots v0.16).


# OPTIONS
**-h**, **\--help**

:   Print help information


**-c**, **\--config** *file*

:   Path to a config *file*


**-o**, **\--oneshot**

:   Enable oneshot mode

    Exit after a profile has been applied or if no profile was matched


**-s**, **\--socket** *path*

:   Override the default path of the IPC socket


**-T**, **\--timeout** *timeout*

:   Wait for *timeout* milliseconds before processing changes \[default: 0\]


**-V**, **\--version**

:   Print version information


# ENVIRONMENT
**SHIKANE_LOG**

:   Controls at what log level shikane and its modules print messages to stderr.
    Available log levels are *error*, *warn*, *info*, *debug* and *trace*;
    sorted from highest to lowest priority. A lower log level includes messages
    from higher ones. Setting **SHIKANE_LOG** to *trace* will let you see
    everything. Setting it to *off* disables all logging.

    This variable allows filtering by modules and accepts a comma-separated
    list: **SHIKANE_LOG**=*shikane=warn*,*shikane::matching=debug* will only
    show *warn*ings and *error*s from shikane except for the matching module
    where log message of level *debug* and above are shown.

    Note: The logging output and filtering is not a stable interface and may be
    subject to change at any time.


**SHIKANE_LOG_STYLE**

:   Controls if colors and styles with ANSI characters are used for log output.
    Possible values are *auto*, *always* and *never*. Defaults to *auto*.


**SHIKANE_LOG_TIME**

:   Enables logging of prefixed timestamps if set to *1*.


# FILES
shikane reads its configuration from **\$XDG_CONFIG_HOME/shikane/config.toml** by
default. The program exits with an error if no config *file* is found.
The config file format is documented in **shikane**(5).


# BUGS
Hopefully less than 4.


# AUTHORS
Hendrik Wolff <hendrik.wolff@agdsn.me>


# SEE ALSO
**shikane**(5), **shikanectl**(1)
