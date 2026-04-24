package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.now
import app.musicopy.ui.screens.TransferScreen
import uniffi.musicopy.ClientModel
import uniffi.musicopy.ClientStateModel

@Composable
fun MobileTransferScreenshot() {
    val clientModel = ClientModel(
        name = "Desktop",
        endpointId = demoEndpointId,
        connectedAt = now(),
        state = ClientStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        index = emptyList(),
        transferJobs = screenshotTransferJobs,
        paused = false,
    )

    TransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = clientModel,
        onBack = {},
        onPause = {},
        onTransferMore = {},
        onDone = {},
    )
}
