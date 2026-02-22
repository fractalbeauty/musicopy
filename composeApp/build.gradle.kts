import gobley.gradle.GobleyHost
import gobley.gradle.cargo.dsl.jvm
import gobley.gradle.rust.targets.RustAndroidTarget
import gobley.gradle.rust.targets.RustTarget
import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.compose)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.compose.hotreload)
    alias(libs.plugins.kotlin.serialization)

    alias(libs.plugins.conveyor)
    alias(libs.plugins.buildconfig)

    // Gobley (requires atomicfu)
    alias(libs.plugins.gobleyCargo)
    alias(libs.plugins.gobleyRust)
    alias(libs.plugins.gobleyUniffi)
    alias(libs.plugins.atomicfu)

    // Kotest (requires KSP)
    alias(libs.plugins.kotest)
    alias(libs.plugins.ksp)
}

val appVersionCode = System.getenv("APP_VERSION_CODE")?.toInt() ?: 1

val appVersion = "0.1.7"

version = appVersion
val androidVersionName = appVersion
val desktopVersionName = appVersion

val macosVersionShort = "1.7"
val macosVersionBuild = "1.7"

buildConfig {
    buildConfigField("APP_VERSION", appVersion)
    buildConfigField("BUILD_TIME", System.currentTimeMillis())
}

kotlin {
    androidTarget {
        @OptIn(ExperimentalKotlinGradlePluginApi::class)
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }

    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "ComposeApp"
            isStatic = true
        }
    }

    jvm("desktop")

    // TODO: this was added at some point (maybe for Conveyor?) but breaks building using the Nix flake,
    // maybe because the flake's toolchain doesn't match and it fails to download the correct toolchain.
    // Commented out for now but might be needed for something.
    // jvmToolchain {
    //     languageVersion = JavaLanguageVersion.of(21)
    //     vendor = JvmVendorSpec.JETBRAINS
    // }

    sourceSets {
        val desktopMain by getting {
            dependencies {
                implementation(compose.desktop.currentOs)
                implementation(libs.kotlinx.coroutines.swing)
            }
        }
        val desktopTest by getting {
            dependencies {
                // Kotest
                implementation(libs.kotest.runner.junit5)
                implementation(libs.kotest.framework.engine)
                implementation(libs.kotest.assertions.core)
                implementation(libs.kotest.assertions.json)

                implementation(libs.kotlinx.serialization.json)
            }
        }

        androidMain.dependencies {
            implementation(libs.compose.ui.tooling.preview)
            implementation(libs.androidx.activity.compose)
            implementation(libs.androidx.core.splashscreen)
            implementation(libs.play.services.code.scanner)
        }
        commonMain.dependencies {
            implementation(libs.compose.runtime)
            implementation(libs.compose.foundation)
            implementation(libs.compose.material3)
            implementation(libs.compose.ui)
            implementation(libs.compose.components.resources)
            implementation(libs.compose.ui.tooling.preview)
            implementation(libs.androidx.lifecycle.viewmodel)
            implementation(libs.androidx.lifecycle.runtime.compose)
            implementation(libs.androidx.lifecycle.viewmodel.compose)
            implementation(libs.androidx.navigation.compose)
            implementation(libs.composables.composeunstyled.primitives)
            implementation(libs.qrose)
            implementation(libs.multiplatform.settings.no.arg)
            implementation(libs.multiplatform.settings.make.observable)
            implementation(libs.multiplatform.settings.coroutines)
            implementation(libs.multiplatform.settings.test)
        }
    }
}

android {
    namespace = "app.musicopy"
    compileSdk = libs.versions.android.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "app.musicopy"
        minSdk = libs.versions.android.minSdk.get().toInt()
        targetSdk = libs.versions.android.targetSdk.get().toInt()
        versionCode = appVersionCode
        versionName = androidVersionName
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    debugImplementation(libs.compose.ui.tooling)

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

            packageVersion = desktopVersionName

            macOS {
                packageVersion = macosVersionShort
                packageBuildVersion = macosVersionBuild
            }
        }
    }
}

val gobleyRustVariant = when (System.getenv("GOBLEY_RUST_VARIANT")) {
    "release" -> gobley.gradle.Variant.Release
    "debug" -> gobley.gradle.Variant.Debug
    else -> null
} ?: gobley.gradle.Variant.Debug
val gobleyRustSkip = System.getenv("GOBLEY_RUST_SKIP") == "true"

cargo {
    // don't install rustup targets automatically
    installTargetBeforeBuild = false

    packageDirectory = layout.projectDirectory.dir("../crates/musicopy")

    jvmVariant = gobleyRustVariant

    // skip if GOBLEY_RUST_SKIP is set, otherwise build desktop for the host target only
    builds.jvm {
        embedRustLibrary = !gobleyRustSkip && (rustTarget == GobleyHost.current.rustTarget)
    }
}

val gobleyUniffiTarget = System.getenv("GOBLEY_UNIFFI_TARGET")?.let {
    RustTarget(it)
} ?: RustAndroidTarget.Arm64
val gobleyUniffiVariant = when (System.getenv("GOBLEY_UNIFFI_VARIANT")) {
    "release" -> gobley.gradle.Variant.Release
    "debug" -> gobley.gradle.Variant.Debug
    else -> null
} ?: gobleyRustVariant

uniffi {
    generateFromLibrary {
        build = gobleyUniffiTarget
        variant = gobleyUniffiVariant
    }
}

// region Work around temporary Compose bugs.
configurations.all {
    attributes {
        // https://github.com/JetBrains/compose-jb/issues/1404#issuecomment-1146894731
        attribute(Attribute.of("ui", String::class.java), "awt")
    }
}
// endregion

// region Kotest setup
tasks.withType<Test>().configureEach {
    useJUnitPlatform()
}
// endregion
