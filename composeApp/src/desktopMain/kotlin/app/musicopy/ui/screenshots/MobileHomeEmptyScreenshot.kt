package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import app.musicopy.AppSettings
import app.musicopy.mockEndpointId
import app.musicopy.now
import app.musicopy.ui.screens.HomeScreen
import uniffi.musicopy.RecentServerModel

@Composable
fun MobileHomeEmptyScreenshot() {
    val appSettings = remember { AppSettings.createMock() }

    LaunchedEffect(true) {
        appSettings.downloadDirectory = "My Music"
        appSettings.downloadDirectoryName = "My Music"
    }

    HomeScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        appSettings = appSettings,
        recentServers = emptyList(),
        connectingTo = null,
        onPickDownloadDirectory = {},
        onConnectQRButtonClicked = {},
        onConnectManuallyButtonClicked = {},
        onConnectRecent = {},
        onShowSettings = {},
    )
}