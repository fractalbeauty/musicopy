import gobley.gradle.GobleyHost
import gobley.gradle.cargo.dsl.jvm
import gobley.gradle.rust.targets.RustAndroidTarget
import gobley.gradle.rust.targets.RustTarget
import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.composeHotReload)
    alias(libs.plugins.serialization)

    alias(libs.plugins.gobleyCargo)
    alias(libs.plugins.gobleyRust)
    alias(libs.plugins.gobleyUniffi)
    kotlin("plugin.atomicfu") version libs.versions.kotlin

    id("dev.hydraulic.conveyor") version "1.12"

    id("com.github.gmazzo.buildconfig") version "5.6.7"
}

val appVersionCode = System.getenv("APP_VERSION_CODE")?.toInt() ?: 1

val appVersion = "0.1.5"

version = appVersion
val androidVersionName = appVersion
val desktopVersionName = appVersion

val macosVersionShort = "1.5"
val macosVersionBuild = "1.5"

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

    jvmToolchain {
        languageVersion = JavaLanguageVersion.of(21)
        vendor = JvmVendorSpec.JETBRAINS
    }

    sourceSets {
        val desktopMain by getting

        androidMain.dependencies {
            implementation(compose.preview)
            implementation(libs.androidx.activity.compose)
            implementation(libs.androidx.core.splashscreen)

            // QR scanner
            implementation("com.google.android.gms:play-services-code-scanner:16.1.0")
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(compose.components.uiToolingPreview)
            implementation(libs.androidx.lifecycle.viewmodel)
            implementation(libs.androidx.lifecycle.runtimeCompose)
            implementation(libs.androidx.lifecycle.viewmodel.compose)

            // bottom sheet
            implementation("com.composables:core:1.36.1")

            // QR generator
            implementation("io.github.alexzhirkevich:qrose:1.0.1")

            // navigation
            implementation("org.jetbrains.androidx.navigation:navigation-compose:2.9.0-beta03")

            // multiplatform settings
            implementation("com.russhwolf:multiplatform-settings-no-arg:1.3.0")
            implementation("com.russhwolf:multiplatform-settings-make-observable:1.3.0")
            implementation("com.russhwolf:multiplatform-settings-coroutines:1.3.0")
            implementation("com.russhwolf:multiplatform-settings-test:1.3.0")
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
        }
        desktopMain.dependencies {
            implementation(compose.desktop.currentOs)
            implementation(libs.kotlinx.coroutinesSwing)
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
    debugImplementation(compose.uiTooling)

    // Conveyor
    linuxAmd64(compose.desktop.linux_x64)
    macAmd64(compose.desktop.macos_x64)
    macAarch64(compose.desktop.macos_arm64)
    windowsAmd64(compose.desktop.windows_x64)
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
