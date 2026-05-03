# Contributing

## Setup

Install:
- Android Studio (also Android NDK)
  - Kotlin Multiplatform plugin
- rustup
- Just

For Android:
- `rustup target add aarch64-linux-android`
- `rustup target add armv7-linux-androideabi`
- `rustup target add i686-linux-android`
- `rustup target add x86_64-linux-android`

For iOS:
- `rustup target add aarch64-apple-ios`
- `rustup target add x86_64-apple-ios`
- `rustup target add aarch64-apple-ios-sim`
- May also need (some of?) the Android targets, since we generate UniFFI bindings from one of the Android builds by default. This could be fixed

## Developing

- Run all tests and checks with `just test`
  - Run Rust tests with `just test-rust` 
  - Run Kotlin tests with `just test-gradle` (open report HTML with `just test-gradle-report`)

### Using the TUI

The `musicopy-tui` crate wraps the `musicopy` crate which contains the core app logic.
Run it with `just run-tui`.
Press `?` for a list of commands, and press `:` to open the command line.

For testing transfers, start two instances and connect them:
- `just run-tui` (will persist the keypair and database to disk)
- `just run-tui -m` to run in-memory
- In the first instance, add a library folder: `:addlibrary music /absolute/path/to/your/music` (`~` will not be expanded)
- In the second instance, copy the endpoint ID and run `:connect <endpoint id>`
- In the first, run `:accept`
- In the second, run `:download 1` to download all files from the first client or `:dlrand 1` to download a random subset of files

## Commit style

Prefix commits with `area:`. Try to use one of the listed areas:
```
core: changes to Rust crates
ui: changes to CMP project
- desktop
- mobile
  - android
  - ios
tui: changes to musicopy-tui crate
web
dev
docs
```
