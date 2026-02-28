set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
  just --list

test:
  just test-rust
  just test-gradle

test-rust *FLAGS:
  cargo nextest run --package musicopy {{FLAGS}}

test-gradle *FLAGS:
  ./gradlew desktopTest {{FLAGS}}

test-gradle-report:
  xdg-open ./composeApp/build/reports/tests/desktopTest/index.html

cov:
  cargo llvm-cov --html nextest --package musicopy

cov-report:
  xdg-open ./target/llvm-cov/html/index.html

download-icon icon:
  curl "https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsoutlined/{{icon}}/default/24px.xml" -o ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/?attr\/colorControlNormal/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/@android:color\/white/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
