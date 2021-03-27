# LAX: Locate Args and Execute

An binary/executable proxy that transforms arguments starting with the "@"
symbol into full directory paths

## Usage

This will open some file "Config.in" with vim, that exists somewhere in the
current directory or the children of the current directory.  

`lax vim @Config.in`  

If there are multiple matches, the user will be prompted to pick one.  

Without the "@" symbol, vim will work as normal:  

`lax vim Config.in`  

And just look for "Config.in" in the current directory

