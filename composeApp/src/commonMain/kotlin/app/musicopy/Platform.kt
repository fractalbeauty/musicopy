package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.ClipEntry
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.TranscodePolicy

expect val isAndroid: Boolean

/**
 * Platform-specific application/process-scoped context.
 */
expect class PlatformAppContext private constructor() {
    val name: String
}

/**
 * Platform-specific activity/scene-scoped context.
 */
expect class PlatformActivityContext private constructor() {}

interface ICoreProvider {
    fun getOptions(platformAppContext: PlatformAppContext): CoreOptions {
        return CoreOptions(
            initLogging = true,
            inMemory = false,
            projectDirs = null,
            transcodePolicy = AppSettings.transcodePolicy
        )
    }
}

expect object CoreProvider : ICoreProvider;

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
