package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.uikit.LocalUIViewController
import kotlinx.cinterop.ExperimentalForeignApi
import platform.Foundation.NSDataBase64EncodingOptions
import platform.Foundation.NSLog
import platform.Foundation.NSURL
import platform.Foundation.NSURLBookmarkCreationMinimalBookmark
import platform.Foundation.NSURLBookmarkCreationWithSecurityScope
import platform.Foundation.base64EncodedStringWithOptions
import platform.Foundation.base64Encoding
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerViewController
import platform.UniformTypeIdentifiers.UTType
import platform.UniformTypeIdentifiers.UTTypeDirectory
import platform.UniformTypeIdentifiers.UTTypeFolder
import platform.darwin.NSObject
import kotlin.collections.listOf

actual class DirectoryPicker internal constructor(val onPick: () -> Unit) {
    actual suspend fun pickDownloadDirectory() {
        onPick()
    }
}

@OptIn(ExperimentalForeignApi::class)
@Composable
actual fun rememberDirectoryPicker(platformContext: PlatformActivityContext): DirectoryPicker {
    val uiViewController = LocalUIViewController.current

    val delegate = remember {
        object : NSObject(), UIDocumentPickerDelegateProtocol {
            override fun documentPicker(
                controller: UIDocumentPickerViewController,
                didPickDocumentsAtURLs: List<*>
            ) {
                if (didPickDocumentsAtURLs.isEmpty()) {
                    return
                }
                val url = didPickDocumentsAtURLs.first() as NSURL

                val bookmarkData = url.bookmarkDataWithOptions(
                    NSURLBookmarkCreationWithSecurityScope, //NSURLBookmarkCreationMinimalBookmark,
                    null,
                    null,
                    null
                )
                if (bookmarkData == null) {
                    return
                }

                AppSettings.downloadDirectory = bookmarkData.base64EncodedStringWithOptions(0uL)
                // TODO: display something nicer
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
