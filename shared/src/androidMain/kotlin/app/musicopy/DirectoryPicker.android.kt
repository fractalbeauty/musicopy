package app.musicopy

import android.content.ContentResolver
import android.content.Intent
import android.net.Uri
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner

class AppLifecycleObserver(
    private val registry: ActivityResultRegistry,
    private val contentResolver: ContentResolver,
    private val appSettings: AppSettings,
) : DefaultLifecycleObserver {
    lateinit var openDocumentTree: ActivityResultLauncher<Uri?>

    override fun onCreate(owner: LifecycleOwner) {
        openDocumentTree =
            registry.register("key", owner, ActivityResultContracts.OpenDocumentTree()) { uri ->
                if (uri == null) {
                    // TODO
                    return@register
                }

                // persist permission
                val modeFlags =
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or
                        Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                contentResolver.takePersistableUriPermission(uri, modeFlags)

                // store uri
                appSettings.downloadDirectory = uri.toString()

                // Extract display name for selected folder:
                // content://.../primary%3ADownload%2FMusicopy --> Download/Musicopy
                val folderName =
                    uri.lastPathSegment
                        ?.substringAfter(':')
                        ?.takeIf { it.isNotEmpty() }
                        ?: "Selected folder"
                appSettings.downloadDirectoryName = folderName
            }
    }
}

actual class DirectoryPicker {
    private var observer: AppLifecycleObserver

    internal constructor(platformContext: PlatformActivityContext) {
        this.observer = platformContext.observer
    }

    actual suspend fun pickDownloadDirectory() {
        observer.openDocumentTree.launch(null)
    }
}

@Composable
actual fun rememberDirectoryPicker(
    platformContext: PlatformActivityContext,
    appSettings: AppSettings,
) = remember { DirectoryPicker(platformContext) }
