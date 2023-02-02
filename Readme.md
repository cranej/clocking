# What is it

A tool to help tracking the usage of your daily time.

# Motivation

It's a start to introduce time-hacking into my life - by tracking it honestly.

# Installation

1. Install Rust and Cargo.
2. Clone this repository.
2. From inside the cloned repository, run `cargo install --path .` - this builds the binary and copies it into `$HOME/.cargo/bin`.

# Usage

There are two interfaces included: command line (CLI) and web.

## CLI

Please use `clocking help` and `clocking help <subcommand>` for usage. A basic workflow might be:

1. When you start some activity, run `clocking start` to start tracking the time spent on it. By default it saves the start event and then waits for `Ctrl-D` to finish the started activity.
2. While during the activity, optionally input notes for the activity.
3. When you decide to pause or stop the activity, press `Ctrl-D` to save the finish event, any lines input before `Ctrl-D` will be saved as notes of the event.
4. Run `clocking help report` to see the options to view your activities.

## Web

Run `clocking help server` to see what are the options to start a web server. By default `clocking server` starts a locally bound server: http://localhost:8080 .

The web page should explain itself.

# Wishlist

- [x] bootstarp - clocking the efforts of this project
- [x] basic report
- [x] support daily view
- [x] finish latest unfinished work
- [x] web server mode
- [x] list recent work names when starting clocking
- [x] daily distribution view
- [ ] separated and redesigned report page
- [ ] daily chart
- [ ] report weekly view
- [ ] support filtering item name when reporting
- [ ] add tags for item
- [ ] report by filtering tags
