# NAME
shikane - configuration file


# DESCRIPTION
shikane uses the [TOML] file format for its configuration file and contains an
array of **profile**s. Each **profile** is a table containing an array of
**output** tables.

The words "output", "display" and "monitor" can be used interchangeably. An
"output" refers to a currently connected display and an "**output**" refers to a
table in the config file. The same goes for "mode" and "**mode**" respectively.

shikane selects possible **profile**s automatically at startup and when a change
in the set of currently connected displays occurs.
A **profile** is taken into consideration if every currently connected display
can be matched to at least one **output** and no **output** is unmatched.\
A display matches an **output** if:\

- the **match** parameter matches against a property of the display (see also
  **OUTPUT FIELDS** below)\
- AND the display supports the **mode** that is specified in the **output**
  table.

After assembling a list of possible **profile**s shikane tries to apply them one
after the other until one succeeds or there are no **profile**s left to try.


# FORMAT
**[[profile]]**
:   Starts a new **profile** table. A *name* must be specified.
    See **PROFILE FIELDS** for details.


## PROFILE FIELDS
**name** = \"*name*\"
:   Mandatory.
    Specifies the *name* of the **profile**.

**\[\[profile.output\]\]**
:   Mandatory.
    Adds an **output** table to the **profile**. Contains parameters used to
    configure a display. See **OUTPUT FIELDS** for details.

    On **sway**(1) for example, output *name*s and *description*s can be
    obtained via **swaymsg -t get_outputs**.

**exec** = \[\"*command*\", ...\]
:   Optional.
    An **exec** array contains a set of *command*s that are executed when the
    **profile** was successfully applied. The order of execution is not
    guaranteed to be preserved. The *name* of the **profile** can be retrieved
    from the **\$SHIKANE_PROFILE_NAME** environment variable.


## OUTPUT FIELDS
A display has, among others, the attributes
*name*, *make*, *model*, *serialnumber* and *description*
and may look like this:
"DP-1", "Company Foo", "FooHD-24", "12345678", "Something Foo Something Bar"

**match** = \"*pattern*\" \| \"/*regex*/\"
:   Mandatory.
    This field is either compared as full text with attributes of the display or
    interpreted as a *regex*. In full text comparison, *name*, *make* and
    *model* are compared with *pattern* for equality. If the *pattern* is
    surrounded by "/", then it is interpreted as a *regex*. The regex is
    compared with the concatenation of *name*, *make*, *model*, *serialnumber*
    and *description* with "|" as separator.

**enable** = *true* \| *false*
:   Mandatory.
    Enables or disables the matched display.

**exec** = \[\"*command*\", ...\]
:   Optional.
    An **exec** array contains a set of *command*s that are executed when the
    **profile** was successfully applied. The order of execution is not
    guaranteed to be preserved. The *name* of the display can be retrieved from
    the **\$SHIKANE_OUTPUT_NAME** environment variable.

The **mode**, **position**, **scale**, **transform** and **adaptive_sync**
options will only change the respective properties of the display if they are
specified (e.g. the scaling of a display will not be changed if the **scale**
field is not present).

**mode** = { width = *width*, height = *height*, refresh = *rate*\(, **custom** = *true* \| *false*\) }
:   Optional.
    Configures the matched display to use the matched mode. Modes are a
    combination of *width* and *height* (in pixels) and a refresh *rate* (in Hz)
    that your display can be configured to use. The refresh *rate* may also be a
    floating point number. A display will not be matched against an **output**
    if the display does not support the specified **mode**. A **mode** can
    optionally be declared as **custom**. In this case, shikane will not try to
    find a mode with the same parameters and will just send the provided values
    to the compositor.

**position** = { x = *x*, y = *y* }
:   Optional.
    Places the display at the specified position in the global coordinate space.

**scale** = *factor*
:   Optional.
    Scales the display by the specified floating point *factor*.

**transform** = \"*transform*\"
:   Optional.
    Sets the display transform. May be one of *90*, *180*, *270* for a
    rotation; or *flipped*, *flipped-90*, *flipped-180*, *flipped-270* for a
    flip and a rotation; or *normal* for no transform.

**adaptive_sync** = *true* \| *false*
:   Optional.
    Enables or disables adaptive synchronization for the display (also known as
    VRR, Variable Refresh Rate).


# EXAMPLES
The indentations are not necessary and are used here only to illustrate the
hierarchy.

```toml
[[profile]]
name = "Company Foo with one vertical display"
    [[profile.output]]
    match = "Company Foo"
    enable = true
    mode = { width = 1920, height = 1080, refresh = 50 }
    position = { x = 0, y = 0 }
    scale = 1.3

    [[profile.output]]
    match = "/HDMI-[ABC]-[1-9]/"
    enable = true
    exec = [ "echo This is output $SHIKANE_OUTPUT_NAME" ]
    position = { x = 1920, y = 0}
    transform = "270"
        [profile.output.mode]
        width = 2560
        height = 1440
        refresh = 75

[[profile]]
name = "custom DP-[1-9] flip"
exec = [ "echo This is an unusual display" ]
    [[profile.output]]
    match = "/DP-[1-9]/"
    enable = true
    transform = "flipped"
        [profile.output.position]
        x = 0
        y = 0
        [profile.output.mode]
        width = 2000
        height = 1500
        refresh = 55.194
        custom = true
```

A laptop has a builtin output that is always connected.
```toml
[[profile]]
name = "laptop builtin"
    [[profile.output]]
    match = "eDP-1"
    enable = true

[[profile]]
name = "no builtin + HDMI"
    [[profile.output]]
    match = "eDP-1"
    enable = false

    [[profile.output]]
    match = "HDMI-A-1"
    enable = true
```

On **sway**(1) for example, **exec** can be used to move workspaces to the
desired output:
```toml
[[profile]]
name = "double monitor"
exec = [
    "swaymsg workspace 1, move workspace to eDP-1",
    "swaymsg workspace 2, move workspace to DP-1" ]
output = [
    {match = "eDP-1", enable = true},
    {match = "DP-1", enable = true} ]
```

shikane provides the **\$SHIKANE_PROFILE_NAME** variable in the environment of the
executed processes of the **profile.exec** array and the **\$SHIKANE_OUTPUT_NAME**
variable in the environment of the **output.exec** processes. If you are using
**sway**(1), the last variable is especially useful in conjunction with
[swayws].
```toml
[[profile]]
name = "generic profile"
exec = ["notify-send shikane \"Profile $SHIKANE_PROFILE_NAME has been applied\""]
    [[profile.output]]
    match = "/DP-[1-9]/"
    enable = true
    exec = ["swayws range --numeric 1 5 $SHIKANE_OUTPUT_NAME"]

    [[profile.output]]
    match = "/HDMI-.-[1-9]/"
    enable = true
    exec = ["swayws range --numeric 6 10 $SHIKANE_OUTPUT_NAME"]
```


# SEE ALSO
**shikane**(1), [swayws], [TOML]


[swayws]: https://gitlab.com/w0lff/swayws
[TOML]: https://toml.io/
