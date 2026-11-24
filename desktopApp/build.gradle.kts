import org.jetbrains.compose.desktop.application.dsl.TargetFormat

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.compose)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.compose.hotreload)

    alias(libs.plugins.conveyor)
}

val appVersion: String by project

version = appVersion

// We set this so that Conveyor knows which JVM toolchain to use for packaging the desktop app,
// but we have an option to disable it since it causes problems when using the Nix flake.
val skipJvmToolchain = System.getenv("MUSICOPY_UNSET_JVM_TOOLCHAIN") == "true"
if (!skipJvmToolchain) {
    kotlin {
        jvmToolchain {
            languageVersion = JavaLanguageVersion.of(21)
            vendor = JvmVendorSpec.JETBRAINS
        }
    }
}

dependencies {
    implementation(projects.shared)

    implementation(compose.desktop.currentOs)
    implementation(libs.kotlinx.coroutines.swing)

    implementation(libs.compose.runtime)
    implementation(libs.compose.foundation)
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.composables.composeunstyled.primitives)
    implementation(libs.qrose)

    // Conveyor
    linuxAmd64(libs.compose.desktop.linux.x64)
    macAmd64(libs.compose.desktop.macos.x64)
    macAarch64(libs.compose.desktop.macos.arm64)
    windowsAmd64(libs.compose.desktop.windows.x64)
}

compose.desktop {
    application {
        mainClass = "app.musicopy.MainKt"

        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
            packageName = "Musicopy"

            packageVersion = appVersion
        }
    }
}
