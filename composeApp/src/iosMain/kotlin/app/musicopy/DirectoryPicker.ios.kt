package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.uikit.LocalUIViewController
import kotlinx.cinterop.ExperimentalForeignApi
import platform.Foundation.NSURL
import platform.Foundation.NSURLBookmarkCreationWithSecurityScope
import platform.Foundation.base64EncodedStringWithOptions
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerViewController
import platform.UniformTypeIdentifiers.UTTypeFolder
import platform.darwin.NSObject
import uniffi.musicopy.logError
import kotlin.collections.listOf

actual class DirectoryPicker internal constructor(val onPick: () -> Unit) {
    actual suspend fun pickDownloadDirectory() {
        onPick()
    }
}

@OptIn(ExperimentalForeignApi::class)
@Composable
actual fun rememberDirectoryPicker(
    platformContext: PlatformActivityContext,
    appSettings: AppSettings,
): DirectoryPicker {
    val uiViewController = LocalUIViewController.current

    val delegate = remember(appSettings) {
        object : NSObject(), UIDocumentPickerDelegateProtocol {
            override fun documentPicker(
                controller: UIDocumentPickerViewController,
                didPickDocumentsAtURLs: List<*>,
            ) {
                if (didPickDocumentsAtURLs.isEmpty()) {
                    return
                }
                val url = didPickDocumentsAtURLs.first() as NSURL

                // important: we need to call startAccessing before creating the bookmark
                val accessingSSR = url.startAccessingSecurityScopedResource()

                val bookmarkData = url.bookmarkDataWithOptions(
                    NSURLBookmarkCreationWithSecurityScope,
                    null,
                    null,
                    null
                )
                if (bookmarkData == null) {
                    logError("DirectoryPicker: bookmarkDataWithOptions returned null")
                    return
                }

                if (accessingSSR) {
                    url.stopAccessingSecurityScopedResource()
                }

                val bookmarkString = bookmarkData.base64EncodedStringWithOptions(0uL)
                appSettings.downloadDirectory = bookmarkString

                // Build a display name from the filesystem path by removing:
                // - components with '~' (e.g. "com~apple~CloudDocs")
                // - 36-char components (app group container UUIDs)
                // and taking the last 2 meaningful components
                val path = url.path ?: ""
                val meaningfulComponents = path.split('/').filter { component ->
                    component.isNotEmpty() && !component.contains('~') && component.length != 36
                }
                val folderName =
                    meaningfulComponents.takeLast(2).joinToString("/").takeIf { it.isNotEmpty() }
                        ?: "Selected folder"
                appSettings.downloadDirectoryName = folderName
            }
        }
    }

    val onPick = {
        val documentPicker =
            UIDocumentPickerViewController(
                forOpeningContentTypes = listOf(UTTypeFolder),
                asCopy = false
            )
        documentPicker.delegate = delegate

        uiViewController.presentViewController(
            documentPicker,
            animated = true,
            completion = null
        )
    }

    return remember { DirectoryPicker(onPick) }
}
