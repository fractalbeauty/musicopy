package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.now
import app.musicopy.ui.screens.PreTransferScreen
import uniffi.musicopy.ClientModel
import uniffi.musicopy.ClientStateModel

@Composable
fun MobilePreTransferScreenshot() {
    val clientModel = ClientModel(
        name = "Desktop",
        nodeId = demoNodeId,
        connectedAt = now(),
        state = ClientStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        index = screenshotIndex,
        transferJobs = emptyList()
    )

    PreTransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = clientModel,
        hasDownloadDirectory = true,
        onPickDownloadDirectory = {},
        onDownloadAll = {},
        onDownloadPartial = {},
        onCancel = {}
    )
}