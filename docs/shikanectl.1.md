# NAME
shikanectl - control the shikane daemon


# SYNOPSIS
**shikanectl** \[options\] \<command\>


# OPTIONS
**-h**, **\--help**

:   Print help information


**-s**, **\--socket** *socket*

:   Connect to the specified *socket*


**-V**, **\--version**

:   Print version information


# COMMANDS
**reload** \[*file*\]

:   Reload the daemon configuration file, optionally by providing a different
    *file*.


**switch** *name*

:   Use the given profile temporarily.


**export** \[options\] *name*

:   Export the current display setup as shikane config. Include vendor, model
    and serial number in the searches by default. It is recommended to use a
    meaningful and unique *name* for the new profile.


## EXPORT OPTIONS
**-d**, **\--description**

:   Include the description in the searches


**-n**, **\--name**

:   Include the name in the searches


**-m**, **\--model**

:   Include the model in the searches


**-s**, **\--serial**

:   Include the serial number in the searches


**-v**, **\--vendor**

:   Include the vendor in the searches


# EXAMPLES
Using `shikanectl export` to append the current output setup as a new profile
to your existing configuration. Just replace the profile name with something
meaningful and unique.
```shell
shikanectl export "NEW_PROFILE_NAME" >> $XDG_CONFIG_HOME/shikane/config.toml
```


# BUGS
Hopefully less than 4.


# AUTHORS
Hendrik Wolff <hendrik.wolff@agdsn.me>


# SEE ALSO
**shikane**(1)
