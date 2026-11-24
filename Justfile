set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

opener := if os() == "macos" {
  "open"
} else {
  "xdg-open"
}

default:
  just --list

run-tui *FLAGS:
  cargo run --package musicopy-tui -- {{FLAGS}}

run-desktop:
  ./gradlew :desktopApp:run -DmainClass=app.musicopy.MainKt

run-desktop-hot:
  ./gradlew :desktopApp:hotRun -DmainClass=app.musicopy.MainKt --auto

run-android:
  ./gradlew :androidApp:installDebug
  adb shell am start -n app.musicopy/.MainActivity

[positional-arguments]
run-transcode-check *args:
  cargo run --package musicopy-transcode-check --release -- "$@"

[positional-arguments]
run-example-transcode *args:
  cargo run --package musicopy-transcode --example transcode --release -- "$@"

[positional-arguments]
run-example-hash *args:
  cargo run --package musicopy-transcode --example hash --release -- "$@"

test:
  cargo check --workspace
  cargo fmt --check
  just test-rust
  just test-gradle
  just ktlint

test-rust *FLAGS:
  cargo nextest run --workspace --features musicopy/test-hooks {{FLAGS}}

test-gradle *FLAGS:
  ./gradlew :shared:jvmTest {{FLAGS}}

test-gradle-report:
  {{opener}} ./shared/build/reports/tests/jvmTest/index.html

ktlint:
  ./gradlew ktlintFormat

cov:
  cargo llvm-cov --html nextest --package musicopy --features musicopy/test-hooks

cov-report:
  {{opener}} ./target/llvm-cov/html/index.html

download-icon icon variant="default":
  curl "https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsoutlined/{{icon}}/{{variant}}/24px.xml" -o ./shared/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/?attr\/colorControlNormal/#FFFFFF/g' ./shared/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/@android:color\/white/#FFFFFF/g' ./shared/src/commonMain/composeResources/drawable/{{icon}}_24px.xml

android-size:
  which bundletool || (echo "missing bundletool"; exit 1)
  rm build/bundletool/musicopy.apks build/bundletool/bundletool-get-size.csv || true
  ./gradlew :androidApp:bundleRelease
  bundletool build-apks --bundle androidApp/build/outputs/bundle/release/androidApp-release.aab --output build/bundletool/musicopy.apks
  bundletool get-size total --apks build/bundletool/musicopy.apks > build/bundletool/bundletool-get-size.csv
  cat build/bundletool/bundletool-get-size.csv