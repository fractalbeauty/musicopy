package app.musicopy.ui

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import app.musicopy.shortenNodeId
import app.musicopy.toClipEntry
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.WidgetContainer
import com.composables.core.Dialog
import com.composables.core.DialogPanel
import com.composables.core.DialogState
import com.composables.core.Scrim
import com.composables.core.rememberDialogState
import io.github.alexzhirkevich.qrose.QrData
import io.github.alexzhirkevich.qrose.rememberQrCodePainter
import io.github.alexzhirkevich.qrose.text
import kotlinx.coroutines.runBlocking
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.content_copy_24px
import musicopy_root.musicopy.generated.resources.input_24px
import musicopy_root.musicopy.generated.resources.open_in_new_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.NodeModel
import uniffi.musicopy.ServerStateModel

@Composable
fun ConnectWidget(
    nodeModel: NodeModel,
    showHints: Boolean,
    onAcceptAndTrust: (remoteNodeId: String) -> Unit,
    onAcceptOnce: (remoteNodeId: String) -> Unit,
    onDeny: (remoteNodeId: String) -> Unit,

    modifier: Modifier = Modifier,
) {
    val nextPending = nodeModel.servers.values.find { it.state is ServerStateModel.Pending }

    // TODO: animate
    val title = if (nextPending == null) {
        "CONNECT"
    } else {
        "PENDING CONNECTION"
    }

    WidgetContainer(
        title = title,
        modifier = modifier,
    ) {
        AnimatedContent(
            targetState = nextPending,
            contentKey = { it -> it?.nodeId },
            transitionSpec = {
                // Compare the incoming number with the previous number.
                val targetConnectedAt = targetState?.connectedAt ?: 0u
                val initialConnectedAt = initialState?.connectedAt ?: 0u
                if (targetConnectedAt > initialConnectedAt) {
                    // If the target number is larger, it slides up and fades in
                    // while the initial (smaller) number slides up and fades out.
                    slideInVertically { height -> height } + fadeIn() togetherWith
                            slideOutVertically { height -> -height } + fadeOut()
                } else {
                    // If the target number is smaller, it slides down and fades in
                    // while the initial number slides down and fades out.
                    slideInVertically { height -> -height } + fadeIn() togetherWith
                            slideOutVertically { height -> height } + fadeOut()
                }.using(
                    // Disable clipping since the faded slide-in/out should
                    // be displayed out of bounds.
                    SizeTransform(clip = false)
                )
            },
        ) { targetState ->
            targetState?.let {
                PendingScreen(
                    remoteNodeId = targetState.nodeId,
                    remoteNodeName = targetState.name,
                    onAcceptAndTrust = { onAcceptAndTrust(targetState.nodeId) },
                    onAcceptOnce = { onAcceptOnce(targetState.nodeId) },
                    onDeny = { onDeny(targetState.nodeId) },
                )
            } ?: run {
                DefaultScreen(
                    localNodeId = nodeModel.nodeId,
                    showHints = showHints
                )
            }
        }
    }
}

@Composable
private fun DefaultScreen(
    localNodeId: String,
    showHints: Boolean,
) {
    val downloadAppState = rememberDialogState(initiallyVisible = false)
    DownloadAppDialog(
        state = downloadAppState,
        onClose = {
            downloadAppState.visible = false
        }
    )

    val enterManuallyState = rememberDialogState(initiallyVisible = false)
    EnterManuallyDialog(
        state = enterManuallyState,
        localNodeId = localNodeId,
        onClose = {
            enterManuallyState.visible = false
        }
    )

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        if (showHints) {
            Info {
                Text(
                    "Scan the QR code using the mobile app to connect.",
                    style = MaterialTheme.typography.bodyMedium
                )
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 16.dp),
            horizontalArrangement = Arrangement.Center
        ) {
            Image(
                painter = rememberQrCodePainter(
                    QrData.text(localNodeId)
                ),
                contentDescription = "QR code containing node ID",
                modifier = Modifier.widthIn(max = 120.dp)
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            TextButton(
                onClick = {
                    downloadAppState.visible = true
                },
            ) {
                Text(
                    "Download app",
                    modifier = Modifier.padding(end = 4.dp)
                )

                Icon(
                    painter = painterResource(Res.drawable.open_in_new_24px),
                    contentDescription = null,
                    modifier = Modifier.size(16.dp)
                )
            }

            TextButton(
                onClick = {
                    enterManuallyState.visible = true
                },
            ) {
                Icon(
                    painter = painterResource(Res.drawable.input_24px),
                    contentDescription = null,
                    modifier = Modifier.size(16.dp)
                )

                Text(
                    "Enter manually",
                    modifier = Modifier.padding(start = 4.dp)
                )
            }
        }
    }
}

@Composable
private fun PendingScreen(
    remoteNodeId: String,
    remoteNodeName: String,
    onAcceptAndTrust: () -> Unit,
    onAcceptOnce: () -> Unit,
    onDeny: () -> Unit,
) {
    var trust by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Box {}

        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(remoteNodeName, style = MaterialTheme.typography.titleMedium)
            Text(shortenNodeId(remoteNodeId), style = MaterialTheme.typography.titleSmall)
        }

        Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = {
                    if (trust) {
                        onAcceptAndTrust()
                    } else {
                        onAcceptOnce()
                    }
                }) {
                    Text("Allow")
                }
                OutlinedButton(onClick = onDeny) {
                    Text("Deny")
                }
            }

            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = trust, onCheckedChange = { trust = it })
                Text("Remember this device", style = MaterialTheme.typography.labelLarge)
            }
        }
    }
}

@Composable
private fun DownloadAppDialog(
    state: DialogState,
    onClose: () -> Unit,
) {
    Dialog(state = state, onDismiss = onClose) {
        Scrim()
        DialogPanel(
            modifier = Modifier
                .widthIn(max = 500.dp)
                .padding(16.dp)
        ) {
            Card(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(32.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Text(
                        text = "Download mobile app",
                        style = MaterialTheme.typography.headlineSmall,
                    )

                    Box(
                        modifier = Modifier.fillMaxWidth().padding(top = 20.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Image(
                            painter = rememberQrCodePainter(
                                QrData.text("https://musicopy.app/download")
                            ),
                            contentDescription = "QR code containing download link",
                            modifier = Modifier.widthIn(max = 120.dp)
                        )
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.Center
                    ) {
                        val uriHandler = LocalUriHandler.current

                        TextButton(
                            onClick = {
                                uriHandler.openUri("https://musicopy.app/download")
                            },
                        ) {
                            Text(
                                text = "musicopy.app/download",
                                modifier = Modifier.padding(end = 4.dp)
                            )

                            Icon(
                                painter = painterResource(Res.drawable.open_in_new_24px),
                                contentDescription = null,
                                modifier = Modifier.size(16.dp)
                            )
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.End),
                    ) {
                        TextButton(
                            onClick = onClose,
                        ) {
                            Text("Done")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun EnterManuallyDialog(
    state: DialogState,
    localNodeId: String,
    onClose: () -> Unit,
) {
    Dialog(state = state, onDismiss = onClose) {
        Scrim()
        DialogPanel(
            modifier = Modifier
                .widthIn(max = 500.dp)
                .padding(16.dp)
        ) {
            Card(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(32.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Text(
                        text = "Connect manually",
                        style = MaterialTheme.typography.headlineSmall,
                    )

                    Text(
                        text = "This code can be used to connect manually to this device.",
                        style = MaterialTheme.typography.bodyMedium
                    )

                    val split = localNodeId.chunked(32).joinToString("\n")

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(
                            8.dp,
                            Alignment.CenterHorizontally
                        ),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(text = split, style = MaterialTheme.typography.monospaceMedium)

                        val clipboard = LocalClipboard.current

                        IconButton(
                            onClick = {
                                runBlocking {
                                    val clip = toClipEntry(localNodeId)
                                    clipboard.setClipEntry(clip)
                                    // not supported in CMP
                                    // Toast.makeText(context, "Copied to clipboard", Toast.LENGTH_SHORT).show()
                                }
                            },
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "Copy button"
                            )
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.End),
                    ) {
                        TextButton(
                            onClick = onClose,
                        ) {
                            Text("Done")
                        }
                    }
                }
            }
        }
    }
}
