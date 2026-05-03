package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import uniffi.musicopy.ClientModel
import uniffi.musicopy.ClientStateModel
import uniffi.musicopy.CounterModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemDownloadStatusModel
import uniffi.musicopy.IndexItemModel
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.LibraryRootModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.ServerModel
import uniffi.musicopy.ServerStateModel
import uniffi.musicopy.StatsModel
import uniffi.musicopy.TransferJobModel
import uniffi.musicopy.TransferJobProgressModel
import kotlin.time.Clock
import kotlin.time.ExperimentalTime

inline fun <T> T.letIf(condition: Boolean, block: (T) -> T): T =
    if (condition) this.let(block) else this

@Composable
fun <T> rememberPoll(
    intervalMs: Long = 100,
    callback: () -> T,
): State<T> {
    val state = remember { mutableStateOf(callback()) }
    LaunchedEffect(callback) {
        while (isActive) {
            state.value = (callback())
            delay(intervalMs)
        }
    }
    return state
}

fun shortenEndpointId(endpointId: String): String {
    return "${endpointId.slice(0..<6)}...${endpointId.slice((endpointId.length - 6)..<(endpointId.length))}"
}

fun formatSize(
    fileSizeModel: FileSizeModel,
    decimals: Int = 1,
): String = formatSize(
    when (fileSizeModel) {
        is FileSizeModel.Actual -> fileSizeModel.v1
        is FileSizeModel.Estimated -> fileSizeModel.v1
        FileSizeModel.Unknown -> 0uL
    }, estimated = fileSizeModel is FileSizeModel.Estimated, decimals = decimals
)

fun formatSize(
    size: ULong,
    estimated: Boolean = false,
    decimals: Int = 1,
): String = formatSize(
    size.toFloat(),
    estimated,
    decimals
)

fun formatSize(
    size: Float,
    estimated: Boolean = false,
    decimals: Int = 1,
): String {
    val estimatedString = if (estimated) {
        "~"
    } else {
        ""
    }

    if (size > 1_000_000_000f) {
        val sizeGB = size / 1_000_000_000f
        return "${estimatedString}${formatFloat(sizeGB, decimals)} GB"
    } else {
        val sizeMB = size / 1_000_000f
        return "${estimatedString}${formatFloat(sizeMB, decimals)} MB"
    }
}

fun mockEndpointId(): String {
    val allowedChars = ('a'..'f') + ('0'..'9')
    return (1..64)
        .map { allowedChars.random() }
        .joinToString("")
}

fun mockNodeModel(
    endpointId: String = mockEndpointId(),
    homeRelay: String = "https://use1-1.relay.iroh.network./",
    servers: List<ServerModel> = emptyList(),
    clients: List<ClientModel> = emptyList(),
): NodeModel {
    return NodeModel(
        endpointId = endpointId,
        homeRelay = homeRelay,
        sendIpv4 = 12345u,
        sendIpv6 = 12345u,
        sendRelay = 12345u,
        recvIpv4 = 12345u,
        recvIpv6 = 12345u,
        recvRelay = 12345u,
        connSuccess = 4u,
        connDirect = 3u,
        servers = servers.associateBy { it.endpointId },
        clients = clients.associateBy { it.endpointId },
        trustedNodes = emptyList(),
        recentServers = emptyList(),
    )
}

fun mockServerModel(
    endpointId: String = mockEndpointId(),
    transferJobs: List<TransferJobModel> = emptyList(),
): ServerModel {
    return ServerModel(
        name = "My Phone",
        endpointId = endpointId,
        connectedAt = now(),
        state = ServerStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        transferJobs = transferJobs
    )
}

fun mockClientModel(
    transferJobs: List<TransferJobModel> = buildList {
        repeat(100) {
            add(mockTransferJobModel(progress = mockTransferJobProgressModelRequested()))
            add(mockTransferJobModel(progress = mockTransferJobProgressModelTranscoding()))
            add(mockTransferJobModel(progress = mockTransferJobProgressModelReady()))
            add(mockTransferJobModel(progress = mockTransferJobProgressModelInProgress()))
            add(mockTransferJobModel(progress = mockTransferJobProgressModelFinished()))
            add(mockTransferJobModel(progress = mockTransferJobProgressModelFailed()))
        }
    },
    paused: Boolean = false,
): ClientModel {
    val endpointId = mockEndpointId()

    return ClientModel(
        name = "My Desktop",
        endpointId = mockEndpointId(),
        connectedAt = now(),
        state = ClientStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        index = listOf(
            // basic example
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/e"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/e"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/e"),
            mockIndexItemModel(endpointId = endpointId, root = "one", basePath = "/e"),

            // folder collapsing example
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b/c"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b/c"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/b/c"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/d"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/d"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "two",
                basePath = "/a/foo/bar/baz/d"
            ),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(endpointId = endpointId, root = "two", basePath = "/e/foo/bar/baz"),

            // a more realistic example
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(endpointId = endpointId, root = "ex", basePath = "/gen2/art4/alb2"),

            // root collapsing example
            mockIndexItemModel(endpointId = endpointId, root = "three", basePath = "/a/b/c/d"),
            mockIndexItemModel(endpointId = endpointId, root = "three", basePath = "/a/b/c/d"),
            mockIndexItemModel(endpointId = endpointId, root = "three", basePath = "/a/b/c/d"),

            // long text example
            mockIndexItemModel(
                endpointId = endpointId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),

            // deep nesting example
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b/c"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b/c/d"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b/c/d/e"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b/c/d/e/f"),
            mockIndexItemModel(endpointId = endpointId, root = "five", basePath = "/a/b/c/d/e/f/g"),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "five",
                basePath = "/a/b/c/d/e/f/g/h"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "five",
                basePath = "/a/b/c/d/e/f/g/h/i"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "five",
                basePath = "/a/b/c/d/e/f/g/h/i/j"
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "five",
                basePath = "/a/b/c/d/e/f/g/h/i/j/k"
            ),

            // download status examples
            mockIndexItemModel(
                endpointId = endpointId,
                root = "six",
                basePath = "",
                downloadStatus = IndexItemDownloadStatusModel.WAITING
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "six",
                basePath = "",
                downloadStatus = IndexItemDownloadStatusModel.IN_PROGRESS
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "six",
                basePath = "",
                downloadStatus = IndexItemDownloadStatusModel.DOWNLOADED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "six",
                basePath = "",
                downloadStatus = IndexItemDownloadStatusModel.FAILED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "six",
                basePath = "",
                downloadStatus = null
            ),

            // download status folders
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting",
                downloadStatus = IndexItemDownloadStatusModel.WAITING
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting",
                downloadStatus = IndexItemDownloadStatusModel.WAITING
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/downloaded",
                downloadStatus = IndexItemDownloadStatusModel.DOWNLOADED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/downloaded",
                downloadStatus = IndexItemDownloadStatusModel.DOWNLOADED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/failed",
                downloadStatus = IndexItemDownloadStatusModel.FAILED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/failed",
                downloadStatus = IndexItemDownloadStatusModel.FAILED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting_failed",
                downloadStatus = IndexItemDownloadStatusModel.WAITING
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting_failed",
                downloadStatus = IndexItemDownloadStatusModel.FAILED
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting_inprogress",
                downloadStatus = IndexItemDownloadStatusModel.WAITING
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/waiting_inprogress",
                downloadStatus = IndexItemDownloadStatusModel.IN_PROGRESS
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/inprogress_downloaded",
                downloadStatus = IndexItemDownloadStatusModel.IN_PROGRESS
            ),
            mockIndexItemModel(
                endpointId = endpointId,
                root = "seven",
                basePath = "/inprogress_downloaded",
                downloadStatus = IndexItemDownloadStatusModel.DOWNLOADED
            ),
        ),
        transferJobs = transferJobs,
        paused = paused,
    )
}

var nextMockIndexItemCount: Int = 1

fun mockIndexItemModel(
    endpointId: String = mockEndpointId(),
    root: String = "library",
    basePath: String = "/a/b/c",
    downloadStatus: IndexItemDownloadStatusModel? = null,
): IndexItemModel {
    val itemCount = nextMockIndexItemCount++

    val estimate = false
    val fileSize = if (estimate) {
        when (itemCount % 10) {
            0 -> FileSizeModel.Unknown
            in 1..2 -> FileSizeModel.Estimated(10000000u)
            else -> FileSizeModel.Actual(12345678u)
        }
    } else {
        FileSizeModel.Actual(12345678u)
    }

    return IndexItemModel(
        endpointId = endpointId,
        root = root,
        path = "${basePath}/file${itemCount}.flac",

        fileSize = fileSize,

        downloadStatus = downloadStatus,
    )
}

var nextMockJobId: ULong = 0u

fun mockTransferJobModel(
    fileRoot: String = "root",
    filePath: String = "a/b/c.mp3",
    fileSize: ULong = 12345678u,
    progress: TransferJobProgressModel = mockTransferJobProgressModelInProgress(),
): TransferJobModel {
    return TransferJobModel(
        jobId = nextMockJobId++,
        fileRoot = fileRoot,
        filePath = filePath,
        fileSize = fileSize,
        progress = progress
    )
}

fun mockTransferJobProgressModelRequested() = TransferJobProgressModel.Requested

fun mockTransferJobProgressModelTranscoding() = TransferJobProgressModel.Transcoding

fun mockTransferJobProgressModelReady() = TransferJobProgressModel.Ready

fun mockTransferJobProgressModelInProgress(
    bytes: ULong = 2345678u,
) = TransferJobProgressModel.InProgress(
    startedAt = now() - 5u,
    bytes = CounterModel(bytes)
)

fun mockTransferJobProgressModelFinished() = TransferJobProgressModel.Finished(
    finishedAt = now() - 1u
)

fun mockTransferJobProgressModelFailed() = TransferJobProgressModel.Failed(
    error = "something went wrong"
)

fun mockLibraryModel(
    localRoots: List<LibraryRootModel> = emptyList(),
    cachedTranscodes: Boolean = true,
    transcoding: Boolean = false,
): LibraryModel {
    return LibraryModel(
        isScanning = false,
        localRoots = localRoots,
        transcodesDir = "~/.cache/musicopy/transcodes",
        transcodesDirSize = if (cachedTranscodes) FileSizeModel.Actual(534_000_000uL) else FileSizeModel.Actual(
            0uL
        ),
        transcodeCountQueued = if (transcoding) CounterModel(27uL) else CounterModel(0uL),
        transcodeCountInprogress = if (transcoding) CounterModel(8uL) else CounterModel(0uL),
        transcodeCountReady = if (transcoding) CounterModel(143uL) else CounterModel(0uL),
        transcodeCountFailed = CounterModel(0uL),
    )
}

fun mockStatsModelWithoutTransfers(): StatsModel {
    return StatsModel(
        launches = 0u,
        serverSessions = 0u,
        clientSessions = 0u,
        serverFiles = 0u,
        clientFiles = 0u,
        serverBytes = 0u,
        clientBytes = 0u
    )
}

fun mockStatsModelWithTransfers(): StatsModel {
    return StatsModel(
        launches = 1u,
        serverSessions = 1u,
        clientSessions = 1u,
        serverFiles = 1u,
        clientFiles = 1u,
        serverBytes = 3_000_000u,
        clientBytes = 3_000_000u,
    )
}


/**
 * Get the current system time in seconds
 */
@OptIn(ExperimentalTime::class)
internal fun now(): ULong {
    return Clock.System.now().epochSeconds.toULong()
}
