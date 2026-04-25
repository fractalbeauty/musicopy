package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.AppSettings
import app.musicopy.mockLibraryModel
import app.musicopy.mockNodeModel
import app.musicopy.mockServerModel
import app.musicopy.mockStatsModelWithoutTransfers
import app.musicopy.mockTransferJobModel
import app.musicopy.mockTransferJobProgressModelFinished
import app.musicopy.mockTransferJobProgressModelInProgress
import app.musicopy.mockTransferJobProgressModelReady
import app.musicopy.mockTransferJobProgressModelTranscoding
import app.musicopy.ui.DesktopHome
import uniffi.musicopy.LibraryRootModel

@Composable
fun DesktopHomeEmptyScreenshot() {
    val appSettings = AppSettings.createMock().apply {
        licenseKey = "placeholder"
    }
    val nodeModel = mockNodeModel(
        endpointId = demoEndpointId,
    )
    val libraryModel = mockLibraryModel(
        cachedTranscodes = false,
        transcoding = false,
    )
    val statsModel = mockStatsModelWithoutTransfers()

    DesktopHome(
        appSettings = appSettings,
        libraryModel = libraryModel,
        nodeModel = nodeModel,
        statsModel = statsModel,
        showHints = false,
        onAcceptAndTrust = {},
        onAcceptOnce = {},
        onDeny = {},
        onAddLibraryRoot = { _: String, _: String -> },
        onRemoveLibraryRoot = {},
        onRescanLibrary = {},
        onDeleteUnusedTranscodes = {},
        onDeleteAllTranscodes = {},
        onUntrustNode = {},

        screenshotHideTopBar = false,
    )
}
