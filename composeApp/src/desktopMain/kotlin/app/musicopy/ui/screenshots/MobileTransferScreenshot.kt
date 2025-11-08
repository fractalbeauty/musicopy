package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.mockClientModel
import app.musicopy.mockTransferJobModel
import app.musicopy.mockTransferJobProgressModelFinished
import app.musicopy.mockTransferJobProgressModelInProgress
import app.musicopy.mockTransferJobProgressModelReady
import app.musicopy.mockTransferJobProgressModelTranscoding
import app.musicopy.ui.screens.TransferScreen

@Composable
fun MobileTransferScreenshot() {
    val clientModel = mockClientModel(
        transferJobs = buildList {
            repeat(7) {
                add(mockTransferJobModel(progress = mockTransferJobProgressModelTranscoding()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelReady()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
            }

            listOf(
                "One" to (1.2 to 2.3),
                "Two" to (0.6 to 3.4),
                "Three" to (3.2 to 4.5),
                "Four" to (2.3 to 3.6),
                "Five" to (2.3 to 4.7),
                "Six" to (2.3 to 2.8),
//                "Seven" to (1.2 to 1.9)
            ).forEach { it ->
                add(
                    mockTransferJobModel(
                        fileRoot = "Favorites",
                        filePath = "foo/bar/${it.first}.mp3",
                        fileSize = (it.second.second * 1_000_000).toULong(),
                        progress = mockTransferJobProgressModelInProgress(
                            bytes = (it.second.first * 1_000_000).toULong()
                        )
                    )
                )

            }

        }
    )

    TransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = clientModel,
        onCancel = {}
    )
}
