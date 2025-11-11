package app.musicopy.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ProgressIndicatorDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.formatFloat
import app.musicopy.isAndroid
import app.musicopy.mockClientModel
import app.musicopy.rememberNotificationsPermission
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.TopBar
import app.musicopy.ui.widgetHeadline
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.check_circle_24px
import musicopy_root.musicopy.generated.resources.chevron_forward_24px
import musicopy_root.musicopy.generated.resources.error_24px
import musicopy_root.musicopy.generated.resources.pending_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import uniffi.musicopy.TransferJobModel
import uniffi.musicopy.TransferJobProgressModel

@Composable
fun TransferScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    onCancel: () -> Unit,
) {
    val jobs = clientModel.transferJobs.sortedBy { it.jobId }

    val inProgressExpanded = remember { mutableStateOf(true) }
    val finishedExpanded = remember { mutableStateOf(false) }
    val failedExpanded = remember { mutableStateOf(true) }
    val waitingExpanded = remember { mutableStateOf(false) }

    val inProgressJobs = jobs.filter { job -> job.progress is TransferJobProgressModel.InProgress }
    val finishedJobs = jobs.filter { job -> job.progress is TransferJobProgressModel.Finished }
    val failedJobs = jobs.filter { job -> job.progress is TransferJobProgressModel.Failed }

    val waitingJobs = jobs.filter { job ->
        job.progress !is TransferJobProgressModel.InProgress &&
                job.progress !is TransferJobProgressModel.Finished &&
                job.progress !is TransferJobProgressModel.Failed
    }

    Scaffold(
        topBar = {
            TopBar(
                title = "Transferring ${clientModel.transferJobs.size} files",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
        ) {
            Column(
                modifier = Modifier.padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                var progress by remember { mutableStateOf(0f) }
                val animatedProgress by animateFloatAsState(
                    targetValue = progress,
                    animationSpec = ProgressIndicatorDefaults.ProgressAnimationSpec
                )

                LaunchedEffect(clientModel.transferJobs) {
                    while (isActive) {
                        val count = clientModel.transferJobs.size
                        val countFinished =
                            clientModel.transferJobs.filter { it.progress is TransferJobProgressModel.Finished }.size

                        if (count != 0) {
                            progress = countFinished.toFloat() / count.toFloat()
                        }

                        delay(100)
                    }
                }

                LinearProgressIndicator(
                    progress = { animatedProgress },
                    modifier = Modifier.fillMaxWidth()
                )
            }

            NotificationPermissionPrompt()

            if (failedJobs.isNotEmpty()) {
                HorizontalDivider(thickness = 1.dp)
            }

            LazyColumn {
                collapsibleSection(
                    title = "FAILED",
                    expanded = failedExpanded,
                    jobs = failedJobs
                )

                collapsibleSection(
                    title = "IN PROGRESS",
                    expanded = inProgressExpanded,
                    jobs = inProgressJobs
                )

                collapsibleSection(
                    title = "WAITING",
                    expanded = waitingExpanded,
                    jobs = waitingJobs
                )

                collapsibleSection(
                    title = "FINISHED",
                    expanded = finishedExpanded,
                    jobs = finishedJobs
                )
            }
        }
    }
}

@Composable
fun TransferJob(job: TransferJobModel) {
    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    formatJobName(job),
                    style = MaterialTheme.typography.labelLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
                Text(
                    formatJobSubtitle(job),
                    style = MaterialTheme.typography.labelMedium
                )
            }

            Box(
                modifier = Modifier.size(30.dp),
                contentAlignment = Alignment.Center
            ) {
                val progress = job.progress
                when (progress) {
                    is TransferJobProgressModel.Requested, is TransferJobProgressModel.Transcoding, is TransferJobProgressModel.Ready -> {
                        Icon(
                            painter = painterResource(Res.drawable.pending_24px),
                            contentDescription = null,
                        )
                    }

                    is TransferJobProgressModel.InProgress -> {
                        var targetProgress by remember { mutableFloatStateOf(0f) }
                        val animatedProgress by animateFloatAsState(
                            targetValue = targetProgress,
                            animationSpec = ProgressIndicatorDefaults.ProgressAnimationSpec,
                        )

                        LaunchedEffect(true) {
                            while (isActive) {
                                targetProgress = job.fileSize?.let {
                                    progress.bytes.get().toFloat() / it.toFloat()
                                } ?: 0f
                                delay(100)
                            }
                        }

                        CircularProgressIndicator(
                            progress = {
                                animatedProgress
                            },
                        )
                    }

                    is TransferJobProgressModel.Finished -> {
                        Icon(
                            painter = painterResource(Res.drawable.check_circle_24px),
                            contentDescription = null,
                        )
                    }

                    is TransferJobProgressModel.Failed -> {
                        Icon(
                            painter = painterResource(Res.drawable.error_24px),
                            contentDescription = null,
                        )
                    }
                }
            }
        }
    }
}

internal fun LazyListScope.collapsibleSection(
    title: String,
    expanded: MutableState<Boolean>,
    jobs: List<TransferJobModel>,
) {
    if (jobs.isEmpty()) {
        return
    }

    item(key = "section-$title") {
        var expanded by expanded
        val degrees by animateFloatAsState(if (expanded) 90f else 0f)

        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp)
                .background(MaterialTheme.colorScheme.primaryContainer)
                .clickable { expanded = !expanded }
                .animateItem()
        ) {
            Row(
                modifier = Modifier.fillMaxSize().padding(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = "$title (${jobs.size})",
                    style = MaterialTheme.typography.widgetHeadline,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                )

                Box(modifier = Modifier.weight(1f))

                Image(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = "Expand icon",
                    modifier = Modifier.rotate(degrees),
                    colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface)
                )
            }
        }
    }

    if (expanded.value) {
        item(key = "spacer-top-$title", contentType = "spacer") {
            Box(modifier = Modifier.height(4.dp).animateItem())
        }

        items(
            items = jobs,
            // key by section so we don't animate between sections
            key = { job -> "job-$title-${job.jobId}" },
            contentType = { "job" }
        ) { job ->
            Box(
                modifier = Modifier
                    .padding(horizontal = 8.dp, vertical = 4.dp)
                    .animateItem()
            ) {
                TransferJob(job)
            }
        }

        item(key = "spacer-bottom-$title", contentType = "spacer") {
            Box(modifier = Modifier.height(4.dp).animateItem())
        }
    }

    item(key = "divider-$title", contentType = "divider") {
        HorizontalDivider(modifier = Modifier.animateItem(), thickness = 1.dp)
    }
}

internal fun formatJobName(job: TransferJobModel): String {
    val pathParts = job.filePath.split("/")
    return "${job.fileRoot}/.../${pathParts.last()}"
}

internal fun formatJobSubtitle(job: TransferJobModel): String {
    val progress = job.progress
    return when (progress) {
        is TransferJobProgressModel.Requested -> {
            "Waiting..."
        }

        is TransferJobProgressModel.Transcoding -> {
            "Transcoding..."
        }

        is TransferJobProgressModel.Ready -> {
            "Waiting..."
        }

        is TransferJobProgressModel.InProgress -> {
            job.fileSize?.let {
                val totalMB = it.toFloat() / 1_000_000f
                val progressMB = progress.bytes.get().toFloat() / 1_000_000f
                "${formatFloat(progressMB, 1)} MB/${formatFloat(totalMB, 1)} MB"
            } ?: "Waiting..."
        }

        is TransferJobProgressModel.Finished -> {
            job.fileSize?.let {
                val totalMB = it.toFloat() / 1_000_000f
                "${formatFloat(totalMB, 1)} MB"
            } ?: ""
        }

        is TransferJobProgressModel.Failed -> "Error: ${progress.error}"
    }
}

@Composable
internal fun NotificationPermissionPrompt() {
    val notificationPermission by rememberNotificationsPermission()

    if (isAndroid && !notificationPermission.isGranted) {
        HorizontalDivider(thickness = 1.dp)

        Box(modifier = Modifier.padding(8.dp)) {
            Info {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Transfers will continue in the background. Enable notifications to see progress and be notified when transfers finish.",
                        style = MaterialTheme.typography.bodyMedium
                    )

                    Button(
                        onClick = { notificationPermission.requestPermission() },
                        modifier = Modifier.fillMaxWidth().height(64.dp),
                        shape = MaterialTheme.shapes.large,
                        contentPadding = PaddingValues(16.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Enable notifications")

                            Icon(
                                painter = painterResource(Res.drawable.chevron_forward_24px),
                                contentDescription = null,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun TransferScreenSandbox() {
    TransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = mockClientModel(),
        onCancel = {}
    )
}
