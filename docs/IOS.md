# iOS notes

## Minimum deployment target

Our minimum deployment target is currently iOS 14.0.0.

It is constrained by:
- Compose Multiplatform 1.11.1 iOS minimum version is iOS 14 ([source](https://kotlinlang.org/docs/multiplatform/compose-compatibility-and-versioning.html))

This needs to be updated:
- For XCode, under Targets > iosApp > Build Settings > Deployment > iOS Deployment Target
- For Rust, in build.gradle.kts > cargo > builds.appleMobile ([source](https://gobley.dev/docs/cross-compilation-tips/#be-aware-of-the-deployment-target-version-on-apple-platforms))

Some usage information is available from https://developer.apple.com/support/app-store.
