package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.ClipEntry
import com.russhwolf.settings.Settings
import uniffi.musicopy.CoreOptions

expect val isAndroid: Boolean

/**
 * Platform-specific application/process-scoped context.
 */
expect class PlatformAppContext {
    val systemDetails: String

    val settingsFactory: Settings.Factory
}

/**
 * Platform-specific activity/scene-scoped context.
 */
expect class PlatformActivityContext

interface ICoreProvider {
    fun getOptions(platformAppContext: PlatformAppContext, appSettings: AppSettings): CoreOptions {
        return CoreOptions(
            initLogging = true,
            inMemory = false,
            projectDirs = null,
        )
    }
}

expect object CoreProvider : ICoreProvider;

expect fun PlatformActivityContext.sendFeedbackEmail(
    description: String,
    logs: ByteArray,
    filename: String,
)

expect fun toClipEntry(string: String): ClipEntry

expect fun formatFloat(f: Float, decimals: Int): String

class PermissionState(
    val isGranted: Boolean,
    val requestPermission: () -> Unit,
)

@Composable
expect fun rememberNotificationsPermission(): MutableState<PermissionState>

@Composable
fun stubRememberNotificationsPermission() =
    remember { mutableStateOf(PermissionState(isGranted = true, requestPermission = {})) }

@Composable
expect fun BackHandler(enabled: Boolean, onBack: () -> Unit)
