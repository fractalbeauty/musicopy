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
fun MobileHomeScreenshot() {
    val appSettings = remember { AppSettings.createMock() }

    LaunchedEffect(true) {
        appSettings.downloadDirectory = "My Music"
        appSettings.downloadDirectoryName = "My Music"
    }

    val recentServers = listOf(
        RecentServerModel(
            // Hardcoded for stability
            endpointId = "7e48cfc6dd1e51352ec629be7ea5333f0b07830ebc8f27bb73cbd7273b2ef038",
            name = "Desktop",
            connectedAt = now() - 10_000uL
        ),
        RecentServerModel(
            // Hardcoded for stability
            endpointId = "41342b6dbe75f5d185fb1cdf2c31c286372cf47391fbc2ece6f5af0f2247f5f3",
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
        onShowSettings = {},
    )
}