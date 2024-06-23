# fakemail

Hacky little tool to generate bulk fake emails for testing.

```
USAGE:
    fakemail [OPTIONS]

OPTIONS:
    -a <ATTACH_DIR>    Directory holding files to randomly attach to messages
    -f <FORMAT>        Output format (mbox, eml) [default: mbox]
    -h, --help         Print help information
    -n <NUM>           Number of emails to generate [default: 1]
    -o <OUTPUT>        Output file for mbox, dir for eml (defaults: stdout/cwd)
    -V, --version      Print version information

```

## Installation

```
$ cargo install --git https://github.com/bcampbell/fakemail
```

## Examples

Write 10000 email files into `bob/INBOX/`:
```
$ fakemail -n 10000 -f eml -o bob/INBOX
```

Write a 10 email mbox out to stdout:

```
$ fakemail -n 10
```


