# json-formatter

A tool for formatting JSON.

## Demo
<image src="https://user-images.githubusercontent.com/23146842/98446856-105e6500-2164-11eb-97f2-107418308b8c.gif" width="800" alt="Demo">

## Install
```console
$ cargo install --path .
... (omit) ...
  Installing /home/yourname/.cargo/bin/jsonfmt
   Installed package `jsonfmt v1.0.0 (/path/to/json-formatter)` (executable `jsonfmt`)
$ jsonfmt --help
USAGE:
    jsonfmt [FLAGS] [json_file]

FLAGS:
    -h, --help        Prints help information
    -i, --in-place    overwrite <json_file> if specified
    -V, --version     Prints version information

ARGS:
    <json_file>
```

## Uninstall
```console
$ cargo uninstall
```
