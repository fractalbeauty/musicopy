package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import app.musicopy.AppSettings
import app.musicopy.mockNodeId
import app.musicopy.now
import app.musicopy.ui.screens.HomeScreen
import uniffi.musicopy.RecentServerModel

@Composable
fun MobileHomeScreenshot() {
    val appSettings = remember { AppSettings.createMock() }

    LaunchedEffect(true) {
        appSettings.downloadDirectory = "My Music"
    }

    val recentServers = listOf(
        RecentServerModel(
            nodeId = mockNodeId(),
            name = "Desktop",
            connectedAt = now() - 10_000uL
        ),
        RecentServerModel(
            nodeId = mockNodeId(),
            name = "Laptop",
            connectedAt = now() - 300_000uL
        ),
    )

    HomeScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        appSettings = appSettings,
        recentServers = recentServers,
        connectingTo = null,
        onPickDownloadDirectory = {},
        onConnectQRButtonClicked = {},
        onConnectManuallyButtonClicked = {},
        onConnectRecent = {},
    )
}