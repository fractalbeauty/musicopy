package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember

actual class DirectoryPicker {
    private var activity: MainActivity

    internal constructor(platformContext: PlatformActivityContext) {
        this.activity = platformContext.mainActivity
    }

    actual suspend fun pickDownloadDirectory() {
        activity.observer.openDocumentTree.launch(null)
    }
}

@Composable
actual fun rememberDirectoryPicker(
    platformContext: PlatformActivityContext,
    appSettings: AppSettings
) = remember { DirectoryPicker(platformContext) }
