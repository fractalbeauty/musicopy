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
import uniffi.musicopy.IndexItemModel
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.LibraryRootModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.ServerModel
import uniffi.musicopy.ServerStateModel
import uniffi.musicopy.TranscodePolicy
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

fun shortenNodeId(nodeId: String): String {
    return "${nodeId.slice(0..<6)}...${nodeId.slice((nodeId.length - 6)..<(nodeId.length))}"
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

fun mockNodeId(): String {
    val allowedChars = ('a'..'f') + ('0'..'9')
    return (1..64)
        .map { allowedChars.random() }
        .joinToString("")
}

fun mockNodeModel(
    nodeId: String = mockNodeId(),
    homeRelay: String = "https://use1-1.relay.iroh.network./",
    servers: List<ServerModel> = emptyList(),
    clients: List<ClientModel> = emptyList(),
): NodeModel {
    return NodeModel(
        nodeId = nodeId,
        homeRelay = homeRelay,
        sendIpv4 = 12345u,
        sendIpv6 = 12345u,
        sendRelay = 12345u,
        recvIpv4 = 12345u,
        recvIpv6 = 12345u,
        recvRelay = 12345u,
        connSuccess = 4u,
        connDirect = 3u,
        servers = servers.associateBy { it.nodeId },
        clients = clients.associateBy { it.nodeId },
        trustedNodes = emptyList(),
        recentServers = emptyList(),
    )
}

fun mockServerModel(
    transferJobs: List<TransferJobModel> = emptyList(),
): ServerModel {
    return ServerModel(
        name = "My Phone",
        nodeId = mockNodeId(),
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
): ClientModel {
    val nodeId = mockNodeId()

    return ClientModel(
        name = "My Desktop",
        nodeId = mockNodeId(),
        connectedAt = now(),
        state = ClientStateModel.Accepted,
        connectionType = "direct",
        latencyMs = 42u,
        index = listOf(
            // basic example
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/a/d"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),
            mockIndexItemModel(nodeId = nodeId, root = "one", basePath = "/e"),

            // folder collapsing example
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/a/foo/bar/baz/d"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),
            mockIndexItemModel(nodeId = nodeId, root = "two", basePath = "/e/foo/bar/baz"),

            // a more realistic example
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art1/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen1/art2/alb"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art3/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb1"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),
            mockIndexItemModel(nodeId = nodeId, root = "ex", basePath = "/gen2/art4/alb2"),

            // root collapsing example
            mockIndexItemModel(nodeId = nodeId, root = "three", basePath = "/a/b/c/d"),
            mockIndexItemModel(nodeId = nodeId, root = "three", basePath = "/a/b/c/d"),
            mockIndexItemModel(nodeId = nodeId, root = "three", basePath = "/a/b/c/d"),

            // long text example
            mockIndexItemModel(
                nodeId = nodeId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),
            mockIndexItemModel(
                nodeId = nodeId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),
            mockIndexItemModel(
                nodeId = nodeId,
                root = "four",
                basePath = "/aaaaaaaaaa/bbbbbbbbbb/cccccccccc/dddddddddd"
            ),

            // deep nesting example
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f/g"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f/g/h"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f/g/h/i"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f/g/h/i/j"),
            mockIndexItemModel(nodeId = nodeId, root = "five", basePath = "/a/b/c/d/e/f/g/h/i/j/k"),

            // downloaded example
            mockIndexItemModel(nodeId = nodeId, root = "six", basePath = "/a", downloaded = true),
            mockIndexItemModel(nodeId = nodeId, root = "six", basePath = "/b", downloaded = true),
            mockIndexItemModel(nodeId = nodeId, root = "six", basePath = "/b", downloaded = false),
        ),
        transferJobs = transferJobs
    )
}

var nextMockIndexItemCount: Int = 1

fun mockIndexItemModel(
    nodeId: String = mockNodeId(),
    root: String = "library",
    basePath: String = "/a/b/c",
    downloaded: Boolean = false,
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
        nodeId = nodeId,
        root = root,
        path = "${basePath}/file${itemCount}.flac",

        fileSize = fileSize,

        downloaded = downloaded,
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
    transcoding: Boolean = false,
): LibraryModel {
    return LibraryModel(
        localRoots = localRoots,
        transcodesDir = "~/.cache/musicopy/transcodes",
        transcodesDirSize = FileSizeModel.Actual(534_000_000uL),
        transcodeCountQueued = if (transcoding) CounterModel(27uL) else CounterModel(0uL),
        transcodeCountInprogress = if (transcoding) CounterModel(8uL) else CounterModel(0uL),
        transcodeCountReady = if (transcoding) CounterModel(143uL) else CounterModel(0uL),
        transcodeCountFailed = CounterModel(0uL),
        transcodePolicy = TranscodePolicy.IF_REQUESTED
    )
}

/**
 * Get the current system time in seconds
 */
@OptIn(ExperimentalTime::class)
internal fun now(): ULong {
    return Clock.System.now().epochSeconds.toULong()
}
