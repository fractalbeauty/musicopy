package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import uniffi.musicopy.CoreException
import uniffi.musicopy.pickFolder

actual class DirectoryPicker {
    internal constructor(platformContext: PlatformActivityContext)

    actual suspend fun pickDownloadDirectory() {
        try {
            val pickedPath = pickFolder()
            AppSettings.downloadDirectory = pickedPath
        } catch (e: CoreException) {
            // TODO: toast?
            println("Error: ${e}")
        }
    }
}

@Composable
actual fun rememberDirectoryPicker(platformContext: PlatformActivityContext) =
    remember { DirectoryPicker(platformContext) }
