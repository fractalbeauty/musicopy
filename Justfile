set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

opener := if os() == "macos" {
  "open"
} else {
  "xdg-open"
}

default:
  just --list

tui *FLAGS:
  cargo run --package musicopy-tui -- {{FLAGS}}

test:
  cargo check --workspace
  cargo fmt --check
  just test-rust
  just test-gradle

test-rust *FLAGS:
  cargo nextest run --package musicopy --features musicopy/test-hooks {{FLAGS}}

test-gradle *FLAGS:
  # Build UniFFI bindings using the host target
  GOBLEY_UNIFFI_TARGET=`rustc -vV | grep 'host:' | cut -d' ' -f2` \
  ./gradlew desktopTest {{FLAGS}}

test-gradle-report:
  {{opener}} ./composeApp/build/reports/tests/desktopTest/index.html

cov:
  cargo llvm-cov --html nextest --package musicopy --features musicopy/test-hooks

cov-report:
  {{opener}} ./target/llvm-cov/html/index.html

download-icon icon variant="default":
  curl "https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsoutlined/{{icon}}/{{variant}}/24px.xml" -o ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/?attr\/colorControlNormal/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/@android:color\/white/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml

android-size:
  which bundletool || (echo "missing bundletool"; exit 1)
  rm build/bundletool/musicopy.apks build/bundletool/bundletool-get-size.csv || true
  ./gradlew :musicopy:bundleRelease
  bundletool build-apks --bundle composeApp/build/outputs/bundle/release/musicopy-release.aab --output build/bundletool/musicopy.apks
  bundletool get-size total --apks build/bundletool/musicopy.apks > build/bundletool/bundletool-get-size.csv
  cat build/bundletool/bundletool-get-size.csv