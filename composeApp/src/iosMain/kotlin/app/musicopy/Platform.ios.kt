package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import com.russhwolf.settings.NSUserDefaultsSettings
import com.russhwolf.settings.Settings
import platform.Foundation.NSNumber
import platform.Foundation.NSNumberFormatter
import platform.UIKit.UIDevice

actual val isAndroid = false

actual class PlatformAppContext {
    actual val name: String =
        UIDevice.currentDevice.systemName() + " " + UIDevice.currentDevice.systemVersion

    actual val settingsFactory: Settings.Factory = NSUserDefaultsSettings.Factory()
}

actual class PlatformActivityContext

actual object CoreProvider : ICoreProvider

@OptIn(ExperimentalComposeUiApi::class)
actual fun toClipEntry(string: String): ClipEntry = ClipEntry.withPlainText(string)

actual fun formatFloat(f: Float, decimals: Int): String {
    val formatter = NSNumberFormatter()
    formatter.minimumFractionDigits = 0u
    formatter.maximumFractionDigits = decimals.toULong()
    formatter.numberStyle = 1u // Decimal
    return formatter.stringFromNumber(NSNumber(f))!!
}

@Composable
actual fun rememberNotificationsPermission(): MutableState<PermissionState> {
    return stubRememberNotificationsPermission()
}

@Composable
actual fun BackHandler(enabled: Boolean, onBack: () -> Unit) {
    // not implemented on iOS
}
