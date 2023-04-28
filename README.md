# Lax

An argument substitution utility intended to make working on projects with
deeply nested directories a bit easier.

"Lax" stands for "Locate Arguments and Execute."

## Basic Usage

Given a binary and an "@" pattern "@foo", Lax will will find the file "foo" and
replace "@foo" with "foo"'s full path, then execute the binary with the new arguments

`lax echo @foo` -> `echo ./foobar/foo`

Multiple "@" patterns are possible:

`lax stat @foo @bar @baz` -> `stat ./foobar/foo ./foobar/target/bar ./foobar/src/baz`

Mixing and matching "@" patterns and normal arguments is also possible:

`lax cat -n @foo bar @baz` -> `cat -n ./foobar/foo bar ./foobar/src/baz`

## Globbing

Globbing is fully supported via [globset](https://docs.rs/globset/0.4.6/globset/).

`lax echo @*.rs` -> `echo ./some/directory/main.rs`

`**` is also supported, but is treated specially. The portion of the pattern
before the first `/**/`(note the surrounding slashes) is the search entry
point. That is, given a pattern like `@foo/**/bar`, Lax will look in the
directory `./foo` for a file that matches `bar`. Any following `**` are handled
normally:

```bash
$ lax echo @foo/**/bar/**/baz
foo/bizz/bazz/bar/beez/baz
```

Will look in directory `./foo` for a path that matches `bar/**/baz`. That is, any
entity `baz` that is a descendent of directory `bar`.

Making use of the search entry point can speed up searches if you know which top-level
subdirectory your query is in, but you don't want to `cd` into it for whatever
reason. It can also be used to specify a path outside your directory.

## Selectors

If there are multiple files matching the given name, Lax will prompt you to choose.
However, you can also specify which file you'd want ahead of time:

```bash
$ lax echo @*.rs^1 # Select the first match
a.rs
$ lax echo @*.rs^2 # Select the second match
b.rs
$ lax echo @*.rs^l # Select the last match. You can also use '-1'
d.rs
$ lax echo @*.rs^-2 # Select the second to last match
c.rs
$ lax echo @*.rs^1,3 # Select the first and third match
a.rs c.rs
$ lax echo @*.rs^a # Select all matches
a.rs b.rs c.rs d.rs
$ lax echo @*.rs^/[ab] # Select with regex
a.rs b.rs
```

Now you know the full syntax for "@" patterns:

`@[SEARCH_ENTRY_POINT/**/]GLOB_PATTERN[^SELECTOR[,SELECTOR]...]`

Where `SEARCH_ENTRY_POINT` is a directory, `GLOB_PATTERN` is a glob pattern,
and `SELECTOR` is `[-n..-1|1..n|'a'|'l']`

## Miscellaneous Features

```bash
# Escape the initial '@' symbol
$ lax echo \\@
@
$ lax echo '\@'
@

# Only match *directories* by adding a forward slash
$ lax echo @foo/
./foo/

# You can also use command-line options to achieve a similar effect
$ lax -d echo @foo
./foo/

# Or only look for files
$ lax -f echo @foo
./tests/foobar/foo

# Or transform a file to its parent
$ lax -fD echo @foo
./tests/foobar

# We also have the ability to specify fallback binaries. This will use `cowsay`
# if it's installed, otherwise it will fallback to `echo`
$ lax 'cowsay|echo' hello
 _______
< hello >
 -------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## Primary Use Case

In your `.bashrc`, you can write `alias vim="lax vim"`

From now on, you can just write:

`vim @foo` -> `vim ./foobar/foo`

This makes working on projects with deep directories, like U-Boot and Yocto,
easier. If the compiler complains about an error in
`my_stupid_little_c_file.c`, you can:

`vim @*stupid*file.c`

## Using With `cd`

You might try to use Lax to do things like:

`lax cd @some_deep_nested_subdirectory`

But you will quickly discover this does not work as intended, as cd is
affecting the current directory of its environment, which belongs to a child
process of the shell.

Instead, you can add this to your `.bashrc`:

```bash
cd(){
    local args;
    if ! args=$(lax -pd -- ${@}); then
        return 1
    fi
    command cd ${args}
}
```

The `-p` flag tells Lax to not execute anything, but simply transform arguments
and print them to stdout. The `-d` flag tells it to only match with directories,
as `cd` has no interest in files. You could also use the `-D` flag if you want
to match files, but `cd` to their parent directory, instead.

## Installing

```bash
$ git clone git@github.com:Property404/lax
...
$ cargo install --path lax
...
```

## License

MIT or Apache-2.0
