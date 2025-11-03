package app.musicopy

import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.BooleanVar
import kotlinx.cinterop.CPointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.value
import platform.Foundation.NSData
import platform.Foundation.NSString
import platform.Foundation.NSURL
import platform.Foundation.NSURLBookmarkCreationMinimalBookmark
import platform.Foundation.NSURLBookmarkCreationWithSecurityScope
import platform.Foundation.NSURLBookmarkResolutionWithSecurityScope
import platform.Foundation.base64EncodedStringWithOptions
import platform.Foundation.create
import platform.Foundation.stringByRemovingPercentEncoding
import uniffi.musicopy.IosBookmarkResolver
import uniffi.musicopy.ResolveBookmarkException
import uniffi.musicopy.setIosBookmarkResolver

fun initBookmarkResolver() {
    val resolver = KotlinIosBookmarkResolver()
    setIosBookmarkResolver(resolver)
}

class KotlinIosBookmarkResolver : IosBookmarkResolver {
    @OptIn(BetaInteropApi::class, ExperimentalForeignApi::class)
    override fun resolveBookmark(bookmark: String): String {
        // parse bookmark base64 string
        val bookmarkData = NSData.create(bookmark, 0uL)
        if (bookmarkData == null) {
            throw ResolveBookmarkException.Unexpected("failed to parse bookmark base64")
        }

        memScoped {
            // resolve bookmark to url
            val isStale: BooleanVar = alloc<BooleanVar>()
            val bookmarkUrl =
                NSURL(
                    byResolvingBookmarkData = bookmarkData,
                    NSURLBookmarkResolutionWithSecurityScope,
                    null,
                    isStale.ptr,
                    null
                )

            // check if stale
            if (isStale.value) {
                // create new bookmark
                val newBookmarkData = bookmarkUrl.bookmarkDataWithOptions(
                    NSURLBookmarkCreationWithSecurityScope, //NSURLBookmarkCreationMinimalBookmark,
                    null,
                    null,
                    null
                )

                // store new bookmark
                if (newBookmarkData != null) {
                    AppSettings.downloadDirectory =
                        newBookmarkData.base64EncodedStringWithOptions(0uL)
                }
            }

            // return file url as string
            return bookmarkUrl.toString()
        }
    }
}
