package app.musicopy.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.unit.dp
import app.musicopy.formatFloat
import app.musicopy.shortenNodeId
import app.musicopy.ui.components.AnimatedList
import app.musicopy.ui.components.ScrollableContainer
import app.musicopy.ui.components.WidgetContainer
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.ServerModel
import uniffi.musicopy.ServerStateModel
import uniffi.musicopy.TransferJobProgressModel

@Composable
fun JobsWidget(
    libraryModel: LibraryModel,
    nodeModel: NodeModel,
) {
    val activeServers = nodeModel.servers.values.filter { it.state is ServerStateModel.Accepted }

    var transcodesVisible by remember { mutableStateOf(false) }
    LaunchedEffect(true) {
        while (isActive) {
            val countQueued = libraryModel.transcodeCountQueued.get()
            val countInProgress = libraryModel.transcodeCountInprogress.get()
            val countFailed = libraryModel.transcodeCountFailed.get()

            transcodesVisible = countQueued > 0uL || countInProgress > 0uL || countFailed > 0uL

            delay(100)
        }
    }

    val visible = activeServers.isNotEmpty() || transcodesVisible

    AnimatedVisibility(visible = visible) {
        WidgetContainer(
            title = "STATUS"
        ) {
            ScrollableContainer { scrollModifier ->

                Column(
                    modifier = Modifier.fillMaxWidth().then(scrollModifier),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    AnimatedVisibility(transcodesVisible) {
                        TranscodeJob(libraryModel)
                    }

                    AnimatedList(
                        activeServers,
                        itemKey = { it.nodeId },
                    ) { connection ->
                        ActiveConnectionJob(
                            connection
                        )
                    }

                    AnimatedList(
                        activeServers.filter { it.transferJobs.any { job -> job.progress !is TransferJobProgressModel.Finished } },
                        itemKey = { it.nodeId },
                    ) { connection ->
                        ActiveTransferJob(connection)
                    }
                }
            }
        }
    }
}

@Composable
private fun TranscodeJob(library: LibraryModel) {
    var countQueued by remember { mutableStateOf(library.transcodeCountQueued.get().toInt()) }
    var countInProgress by remember {
        mutableStateOf(
            library.transcodeCountInprogress.get().toInt()
        )
    }
    var countReady by remember { mutableStateOf(library.transcodeCountReady.get().toInt()) }
    var countFailed by remember { mutableStateOf(library.transcodeCountFailed.get().toInt()) }

    LaunchedEffect(true) {
        while (isActive) {
            countQueued = library.transcodeCountQueued.get().toInt()
            countInProgress = library.transcodeCountInprogress.get().toInt()
            countReady = library.transcodeCountReady.get().toInt()
            countFailed = library.transcodeCountFailed.get().toInt()

            delay(100)
        }
    }

    val countRemaining = countQueued + countInProgress

    if (countRemaining > 0) {
        Job(
            labelLeft = {
                Text(
                    "Transcoding $countRemaining files",
                    style = MaterialTheme.typography.labelLarge
                )
            },
            body = {
                Column {
                    Text(
                        "$countInProgress in progress, $countQueued queued",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
        )
    }

    if (countFailed > 0) {
        Job(
            labelLeft = {
                Text(
                    "Failed to transcode $countFailed files",
                    style = MaterialTheme.typography.labelLarge
                )
            }
        )
    }
}

@Composable
private fun ActiveConnectionJob(connection: ServerModel) {
    Job(
        labelLeft = {
            Text("Connected to ${connection.name}", style = MaterialTheme.typography.labelLarge)
        },
        body = {
            Column {
                Text(
                    "Node ID: ${shortenNodeId(connection.nodeId)}",
                    style = MaterialTheme.typography.bodyMedium
                )
                Text(
                    "Connection Type: ${connection.connectionType}",
                    style = MaterialTheme.typography.bodyMedium
                )
                Text(
                    "Latency: ${connection.latencyMs}ms",
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }
    )
}

@Composable
private fun ActiveTransferJob(connection: ServerModel) {
    val count = connection.transferJobs.size
    val countTranscoding =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.Transcoding }.size
    val countReady =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.Ready }.size
    val countInProgress =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.InProgress }.size
    val countFinished =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.Finished }.size
    val countFailed =
        connection.transferJobs.filter { it.progress is TransferJobProgressModel.Failed }.size

    val countRemaining = countTranscoding + countReady + countInProgress
    val countEnded = countFinished + countFailed

    val progressPercent = countEnded.toFloat() / count.toFloat()
    val progressPercentString = formatFloat(progressPercent * 100, 0)

    Job(
        labelLeft = {
            Text(
                "Transferring $count files to ${connection.name} ($progressPercentString%)",
                style = MaterialTheme.typography.labelLarge
            )
        },
        body = {
            Column {
                Text(
                    "$countFinished finished, $countRemaining remaining",
                    style = MaterialTheme.typography.bodyMedium
                )

                if (countTranscoding > 0) {
                    Text(
                        "$countTranscoding transcoding",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }

                if (countFailed > 0) {
                    Text(
                        "$countFailed failed",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }

                for (job in connection.transferJobs) {
                    val progress = job.progress
                    if (progress is TransferJobProgressModel.Failed) {
                        Text(
                            "${job.fileRoot}/${job.filePath} failed: ${progress.error}",
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            }
        }
    )
}

@Composable
private fun Job(
    labelLeft: @Composable () -> Unit = {},
    labelRight: @Composable () -> Unit = {},
    body: (@Composable () -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    val degrees by animateFloatAsState(if (expanded) 90f else 0f)
    Card(
        modifier = Modifier.fillMaxWidth().then(modifier),
    ) {
        Column {
            Row(
                modifier = Modifier.fillMaxWidth().clip(MaterialTheme.shapes.medium)
                    .clickable { expanded = !expanded }
            ) {
                Row(
                    modifier = Modifier.padding(8.dp, 4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    labelLeft()

                    Box(modifier = Modifier.weight(1f))

                    labelRight()

                    Image(
                        painter = painterResource(Res.drawable.chevron_forward_24px),
                        contentDescription = "Expand icon",
                        modifier = Modifier.rotate(degrees),
                        colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface)
                    )
                }
            }

            AnimatedVisibility(
                visible = expanded,
            ) {
                Box(Modifier.fillMaxWidth().padding(8.dp, 4.dp)) {
                    body?.invoke()
                }
            }
        }
    }
}
