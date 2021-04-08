# LAX: Locate Args and Execute

An argument substitution utility 

## Description

Given an binary and argument "@foo", lax will will find the file "foo" and
replace "@foo" with foo's full path, and execute the binary with the new arguments    

`lax echo @foo` -> `echo ./foobar/foo`  

Multiple @'s are possible:  

`lax stat @foo @bar @baz` -> `stat ./foobar/foo ./foobar/target/bar ./foobar/src/baz`  

Mixing and matching @ arguments and non-@ arguments is also possible:  

`lax cat -n @foo bar @baz` -> `cat -n ./foobar/foo bar ./foobar/src/baz`  

Limited globbing is supported:  

`lax echo *.rs` -> `echo ./some/directory/main.rs`  

If there are multiple files matching the given name, Lax will prompt you to choose.  

## Primary Use Case  

In your `.bashrc`, you can write `alias vim="lax vim"`  

From now on, you can just write:  

`vim @foo` -> `vim ./foobar/foo`  

This makes working on projects with deep directories, like U-Boot and Yocto,
easier. If you want to edit a certain defconfig, you can  

`vim @my_dumb_little_defconfig`  
