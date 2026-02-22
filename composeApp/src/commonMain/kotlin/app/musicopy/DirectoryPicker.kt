package app.musicopy

import androidx.compose.runtime.Composable

expect class DirectoryPicker {
    suspend fun pickDownloadDirectory()
}

@Composable
expect fun rememberDirectoryPicker(
    platformContext: PlatformActivityContext,
    appSettings: AppSettings
): DirectoryPicker
