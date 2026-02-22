# Contributing

## Setup

Install:
- Android Studio (also Android NDK)
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

- Run Rust tests with `just test-rust` 
- Run Kotlin tests with `just test-gradle` (open report HTML with `just test-gradle-report`)
