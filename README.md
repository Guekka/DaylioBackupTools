# Daylio Backup Merger

## Description

Daylio is a diary tool. It provides backups, but no way to merge them. We're here to fix that

This tool only merges entries, tags and moods

**Disclaimer**: I have only tried this tool on my backups. It may or may not work for you. It only has been tried on
version 15, as of 2023/01/18

## How to use

*Note*: This tool has only been tested on Linux. It is possible to use it on Windows but will require more tweaking to
get poppler to work.

Install [rust](https://www.rust-lang.org/tools/install).

Download the code repository. Open a terminal in the root directory and run the following command

```sh
cargo run -- merge <main.daylio> <new.daylio> <out.daylio>
```

- `main.daylio` is the main file. Settings, achievements, templates and more will be kept from that file
- `new.daylio` is the file we are adding entries, tags and moods from
- `out.daylio` is the file that will be created with the merged data

