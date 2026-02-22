package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import uniffi.musicopy.CoreOptions
import java.awt.Window
import java.awt.datatransfer.StringSelection
import java.text.DecimalFormat

actual val isAndroid = false

actual class PlatformAppContext actual constructor() {
    actual val name: String = "Java ${System.getProperty("java.version")}"
}

actual class PlatformActivityContext private actual constructor() {
    lateinit var mainWindow: Window
        private set

    constructor(mainWindow: Window) : this() {
        this.mainWindow = mainWindow
    }
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
