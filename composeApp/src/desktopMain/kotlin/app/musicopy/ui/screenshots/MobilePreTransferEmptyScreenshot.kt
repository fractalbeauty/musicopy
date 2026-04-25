package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.now
import app.musicopy.ui.screens.PreTransferScreen
import uniffi.musicopy.ClientModel
import uniffi.musicopy.ClientStateModel

@Composable
fun MobilePreTransferEmptyScreenshot() {
    val clientModel = ClientModel(
        name = "Desktop",
        endpointId = demoEndpointId,
        connectedAt = now(),
        state = ClientStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        index = emptyScreenshotIndex,
        transferJobs = emptyList(),
        paused = false,
    )

    PreTransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = clientModel,
        hasDownloadDirectory = true,
        onPickDownloadDirectory = {},
        onSetDownloads = {},
        onNavigateToTransfer = {},
        onCancel = {}
    )
}