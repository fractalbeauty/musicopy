package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import com.russhwolf.settings.PreferencesSettings
import com.russhwolf.settings.Settings
import uniffi.musicopy.CoreOptions
import java.awt.Window
import java.awt.datatransfer.StringSelection
import java.text.DecimalFormat

actual val isAndroid = false

actual class PlatformAppContext {
    actual val systemDetails
        get() = buildString {
            appendLine(
                "Platform: ${System.getProperty("os.name")} ${System.getProperty("os.version")} (${
                    System.getProperty(
                        "os.arch"
                    )
                })"
            )
            appendLine("Java: ${System.getProperty("java.version")}")
        }

    actual val settingsFactory: Settings.Factory = PreferencesSettings.Factory()
}

actual class PlatformActivityContext {
    val mainWindow: Window

    constructor(mainWindow: Window) {
        this.mainWindow = mainWindow
    }
}

actual fun PlatformActivityContext.sendFeedbackEmail(
    description: String,
    logs: ByteArray,
    filename: String,
    onError: (Exception) -> Unit,
) {
    throw Exception("Not supported on this platform")
}

actual object CoreProvider : ICoreProvider {
    override fun getOptions(
        platformAppContext: PlatformAppContext,
        appSettings: AppSettings,
    ): CoreOptions {
        val defaults = super.getOptions(platformAppContext, appSettings)
//        defaults.inMemory = true
        return defaults
    }
}

@OptIn(ExperimentalComposeUiApi::class)
actual fun toClipEntry(string: String): ClipEntry = ClipEntry(StringSelection(string))

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}

@Composable
actual fun rememberNotificationsPermission(): MutableState<PermissionState> {
    return stubRememberNotificationsPermission()
}

@Composable
actual fun BackHandler(enabled: Boolean, onBack: () -> Unit) {
    // not implemented on desktop
}
