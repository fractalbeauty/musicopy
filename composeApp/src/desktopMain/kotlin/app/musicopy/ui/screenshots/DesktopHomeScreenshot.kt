package app.musicopy.ui.screenshots

import androidx.compose.runtime.Composable
import app.musicopy.mockLibraryModel
import app.musicopy.mockNodeModel
import app.musicopy.mockServerModel
import app.musicopy.mockTransferJobModel
import app.musicopy.mockTransferJobProgressModelFinished
import app.musicopy.mockTransferJobProgressModelInProgress
import app.musicopy.mockTransferJobProgressModelReady
import app.musicopy.mockTransferJobProgressModelTranscoding
import app.musicopy.ui.DesktopHome
import uniffi.musicopy.LibraryRootModel

@Composable
fun DesktopHomeScreenshot() {
    val nodeModel = mockNodeModel(
        nodeId = "941117ff675f3ac981ed27eb0bef5f32471bbc493fdc7aa4d416e5fa0d99f83a",
        servers = listOf(
            mockServerModel(
                transferJobs = buildList {
                    repeat(7) {
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelTranscoding()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelReady()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelInProgress()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                        add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
                    }
                }
            )
        )
    )
    val libraryModel = mockLibraryModel(
        localRoots = listOf(
            LibraryRootModel(
                name = "Favorites",
                path = "~/music/fav2025",
                numFiles = 83u
            ),
            LibraryRootModel(
                name = "Backlog",
                path = "~/music/backlog",
                numFiles = 427u
            ),
        ),
        transcoding = true,
    )

    DesktopHome(
        libraryModel = libraryModel,
        nodeModel = nodeModel,
        showHints = false,
        onAcceptAndTrust = {},
        onAcceptOnce = {},
        onDeny = {},
        onAddLibraryRoot = { _: String, _: String -> },
        onRemoveLibraryRoot = {},
        onRescanLibrary = {},
        onSetTranscodePolicy = {},
        onDeleteUnusedTranscodes = {},
        onDeleteAllTranscodes = {},
        onUntrustNode = {},

        screenshotHideTopBar = true,
    )
}
