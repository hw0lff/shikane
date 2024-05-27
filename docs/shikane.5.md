# NAME
shikane - configuration file


# DESCRIPTION
shikane uses the [TOML] file format for its configuration file and contains an
array of **profile**s. Each **profile** is a table containing an array of
**output** tables.

The words "output", "display" and "monitor" can be used interchangeably. In the
text below, however, a distinction is made between "display" and "**output**".
A "display" refers to a currently connected monitor and an "**output**" refers
to a table in the config file. The same goes for "mode" and "**mode**"
respectively. (Please note the font weight.)

shikane selects possible **profile**s automatically at startup and when a change
in the set of currently connected displays occurs.
A **profile** is taken into consideration if every currently connected display
can be matched to at least one **output** and no **output** is unmatched.\
A display matches an **output** if:\

- the **search** parameter matches against the properties of the display (see
  also **OUTPUT FIELDS** below)\
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
(display,**output**,mode)-combinations are possible, shikane generates variants
with all of them.

You don't need to write the configuration file by hand. You can use any tool to
arrange the displays however you want and then use the **export** command of
**shikanectl**(1). It will generate the config for you.


# FORMAT
**timeout** = *timeout*

:   Optional.
    shikane will wait the specified *timeout* (in milliseconds) whenever a
    change is detected. Usually you should not set this as it slows down
    shikane.
    Is by default *0*.


**\[\[profile\]\]**

:   Optional.
    Starts a new **profile** table. A *name* must be specified.
    See **PROFILE FIELDS** for details.


## PROFILE FIELDS
**name** = \"*name*\"

:   Mandatory.
    Specifies the *name* of the **profile**.


**\[\[profile.output\]\]**

:   Mandatory.
    Adds an **output** table to the **profile**. Contains parameters used to
    configure a display. See **OUTPUT FIELDS** for details.

    On **sway**(1) for example, display *name*s and *description*s can be
    obtained via **swaymsg -t get_outputs**.


**exec** = \[ \"*command*\", ... \]

:   Optional.
    An **exec** array contains a set of *command*s that are executed when the
    **profile** was successfully applied. The order of execution is not
    guaranteed to be preserved. The *name* of the **profile** can be retrieved
    from the **\$SHIKANE_PROFILE_NAME** environment variable.


## OUTPUT FIELDS
A display has, among others, the attributes
*name*, *vendor*, *model*, *serialnumber* and *description*
and may look like this:
"DP-1", "Company Foo", "FooHD-24", "12345678", "Something Foo Something Bar"

**search** = \"*pattern*\"

**search** = \"\[*kind*\]*pattern*\"

**search** = \"\[\[*attrs*\]*kind*\]*pattern*\"

**search** = \[ \"\[\[*attrs*\]*kind*\]*pattern*\", ... \]

:   Mandatory.
    This field consists of 3 parts. The **attribute list** *attrs* at the
    beginning, the **search kind** *kind* in the middle, and the *pattern* at
    the end.

    The **attribute list** *attrs* defines which attributes will be compared
    with the *pattern* and how they are weighted.
    It is a sequence of letters from the set {**dnmvs**}.
    The letters correspond to the first letters of the above listed attributes.
    Each attribute should not be used more than once per **search** field.
    The first attribute has the highest/best weight,
    the last attribute has the lowest/worst weight.
    For a single **search** to be considered applicable,
    all attributes must match the *pattern*.

    The **search kind** *kind* can be one of {**=/%**}.
    The 3 **search kinds** — regex(**/**), substring(**%**), full text(**=**) —
    define how the *pattern* is compared with each **attribute** in the *attrs*
    list.
    With full text comparison, the given attributes are compared with the
    *pattern* for equality.
    With substring comparison, the given attributes have to contain the
    *pattern* as a substring.
    The weight is calculated by dividing the lengths of the *pattern* and
    attribute strings.
    Equal lengths will rank the same as full text comparison.
    With regex comparison, the *pattern* is interpreted as a regular expression.
    regex comparison will always rank lower than the other two **search kinds**.

    The **attribute list** and the **search kind** are optional. If unspecified
    **search kind** defaults to full text comparison and shikane tries to find
    at least one matching attribute.

    Alternatively, several searches, up to a maximum of 5, may be specified in
    an array.


**enable** = *true* \| *false*

:   Mandatory.
    Enables or disables the matched display.


**exec** = \[ \"*command*\", ... \]

:   Optional.
    An **exec** array contains a set of *command*s that are executed when the
    **profile** was successfully applied. The order of execution is not
    guaranteed to be preserved. The *name* of the display can be retrieved from
    the **\$SHIKANE_OUTPUT_NAME** environment variable.

</br>

The **mode**, **position**, **scale**, **transform** and **adaptive_sync**
options will only change the respective properties of the display if they are
specified (e.g. the scaling of a display will not be changed if the **scale**
field is not present).

**mode** = \"*best*\" | \"*preferred*\"

**mode** = \"\[\!\]*width*x*height*\[\@*rate*\[Hz\]\]\"

**mode** = { width = *width*, height = *height*, refresh = *rate*\[, custom = *true* \| *false*\] }

:   Optional.
    Configures the matched display to use the matched mode. Modes are a
    combination of *width* and *height* (in pixels) and a refresh *rate* (in
    Hz) that your display can be configured to use. The refresh *rate* may also
    be a floating point number. A display will not be matched against an
    **output** if the display does not support the specified **mode**.

    A parameterized **mode** can optionally be declared as custom by prefixing
    an exclamation mark \(\!\). In this case, shikane will not try to find a
    mode with the same parameters and will just send the provided values to the
    compositor.

    Setting **mode** to *best* or *preferred* instructs shikane to choose the
    mode itself. The *best* mode is determined by shikane as the mode with the
    highest pixel count, width, height and refresh rate. Most displays announce
    a mode that they prefer. If **mode** is set to *preferred*, shikane will
    select the preferred mode if it exists. Otherwise, the best mode is chosen
    as a fallback.


**position** = \"*x*,*y*\"

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
    search = "v=Company Foo"
    enable = true
    mode = "1920x1080@50Hz"
    position = "0,0"
    scale = 1.3

    [[profile.output]]
    search = "n/HDMI-[ABC]-[1-9]"
    enable = true
    exec = [ "echo This is output $SHIKANE_OUTPUT_NAME" ]
    position = "1920,0"
    transform = "270"
        [profile.output.mode]
        width = 2560
        height = 1440
        refresh = 75

[[profile]]
name = "custom DP-[1-9] flip"
exec = [ "echo This is an unusual display" ]
    [[profile.output]]
    search = "/DP-[1-9]"
    enable = true
    transform = "flipped"
    position = "0,0"
    mode = "!2000x1500@55.194Hz"
```

A laptop has a builtin display that is always connected.
```toml
[[profile]]
name = "laptop builtin"
    [[profile.output]]
    search = "eDP-1"
    enable = true

[[profile]]
name = "no builtin + HDMI"
    [[profile.output]]
    search = "n=eDP-1"
    enable = false

    [[profile.output]]
    search = "n=HDMI-A-1"
    mode = "best"
    enable = true
```

shikane allows you to specifiy multiple searches. This way profiles can be as
specific as possible.
```toml
[[profile]]
name = "home setup"
    [[profile.output]]
    search = "n=eDP-1"
    enable = false

    [[profile.output]]
    search = [ "n=HDMI-A-1", "s=1234VBAM", "m=1QX04Z", "v=specific company" ]
    enable = true

    [[profile.output]]
    search = "smv=1234abcd"
    enable = true
```

On **sway**(1) for example, **exec** can be used to move workspaces to the
desired display:
```toml
[[profile]]
name = "double monitor"
exec = [
    "swaymsg workspace 1, move workspace to eDP-1",
    "swaymsg workspace 2, move workspace to DP-1" ]
output = [
    {search = "eDP-1", enable = true},
    {search = "DP-1", enable = true} ]
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
    search = "/DP-[1-9]"
    enable = true
    exec = ["swayws range --numeric 1 5 $SHIKANE_OUTPUT_NAME"]

    [[profile.output]]
    search = "n%HDMI-"
    enable = true
    mode = "preferred"
    exec = ["swayws range --numeric 6 10 $SHIKANE_OUTPUT_NAME"]
```


# AUTHORS
Hendrik Wolff <hendrik.wolff@agdsn.me>


# SEE ALSO
**shikane**(1), **shikanectl**(1), [swayws], [TOML]


[swayws]: https://gitlab.com/w0lff/swayws
[TOML]: https://toml.io/
