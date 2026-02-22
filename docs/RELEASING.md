# Releasing

Release process:
- Bump versions in `composeApp/build.gradle.kts` according to Versioning
- Commit
- Tag? (we should start doing this, and maybe automatically release on tag)

For desktop releases, we use the [desktop workflow][desktop-workflow] in GitHub Actions. This is
manually triggered for the main branch after pushing a release commit. The action builds the desktop
app using [Conveyor][conveyor] and uploads the built artifacts to object storage, including the
[download page][desktop-download] and the manifests for automatic updates.

For Android, we use the [Android workflow][android-workflow] in GitHub Actions. This is manually
triggered for the main branch after pushing a release commit. The action builds the Android app and
pushes a draft to the Google Play Console (currently, to a closed testing track), which needs to be
manually released.

For iOS, we build the release manually in XCode:
- Product > Archive
- Upload to App Store Connect
- Release TestFlight build in App Store Connect

[desktop-workflow]: https://github.com/fractalbeauty/musicopy/actions/workflows/desktop.yml
[conveyor]: https://conveyor.hydraulic.dev
[desktop-download]: https://download.musicopy.app/download.html
[android-workflow]: https://github.com/fractalbeauty/musicopy/actions/workflows/android.yml

## Versioning

Eventually, we should have one unified `1.x.0` version number that is bumped together for all
platforms. The patch number should only be used for hotfix releases, since it's simpler to have one
incrementing version for everything.

**Android**
- Version name uses unified version
- Version code is `YYMMDDBB` where `BB` is a build counter
- Currently on ~`0.1.7`, need to jump to `1.x` for stable release

**iOS**
- Version uses unified version
- Build number is manually incremented (since we release iOS manually)
- Currently on ~`1.0 (3)` in TestFlight, need to jump to `1.x` for stable release

**Desktop**
- MacOS is currently on ~`1.7`, so we need to advance everything to `1.8` (or the corresponding
  increase) when anything jumps to `1.x`
- Windows/Linux need to jump to `1.x` for stable release
