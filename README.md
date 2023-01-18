# Daylio Backup Merger

## Description

Daylio is a diary tool. It provides backups, but no way to merge them. We're here to fix that

This tool only merges entries, tags and moods

**Disclaimer**: I have only tried this tool on my backups. It may or may not work for you. It only has been tried on version 15, as of 2023/01/18

## How to use

Download Python, at least version 3.10. Run in the terminal
```sh
python main.py <main.daylio> <new.daylio>
```
- `main.daylio` is the main file. Settings, achievements, templates and more will be kept from that file
- `new.daylio` is the file we are adding entries, tags and moods from