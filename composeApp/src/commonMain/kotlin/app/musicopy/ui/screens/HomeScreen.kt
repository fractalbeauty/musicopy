package app.musicopy.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.AppSettings
import app.musicopy.mockNodeId
import app.musicopy.now
import app.musicopy.shortenNodeId
import app.musicopy.ui.components.DetailBox
import app.musicopy.ui.components.DetailItem
import app.musicopy.ui.components.SectionHeader
import app.musicopy.ui.components.TopBar
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_forward_24px
import musicopy_root.musicopy.generated.resources.input_24px
import musicopy_root.musicopy.generated.resources.qr_code_scanner_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.RecentServerModel
import kotlin.time.Duration.Companion.seconds

@Composable
fun HomeScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,

    recentServers: List<RecentServerModel>,
    connectingTo: String?,
    onPickDownloadDirectory: () -> Unit,
    onConnectQRButtonClicked: () -> Unit,
    onConnectManuallyButtonClicked: () -> Unit,
    onConnectRecent: (nodeId: String) -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(title = "Musicopy", onShowNodeStatus = onShowNodeStatus)
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(modifier = Modifier.padding(8.dp)) {
                val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(
                    null
                )

                DetailBox(
                    actionLabel = if (downloadDirectory == null) {
                        "Choose"
                    } else {
                        "Change"
                    },
                    onAction = onPickDownloadDirectory,
                ) {
                    downloadDirectory?.let { downloadDirectory ->
                        DetailItem("Download Folder", downloadDirectory)
                    } ?: run {
                        DetailItem("Download Folder", "Not selected")
                    }
                }
            }

            HorizontalDivider(thickness = 1.dp)

            HomeSection("CONNECT") {
                Row(
                    modifier = Modifier
                        .fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Button(
                        modifier = Modifier
                            .weight(1f)
                            .height(140.dp),
                        shape = MaterialTheme.shapes.large,
                        onClick = onConnectQRButtonClicked,
                    ) {
                        Column(
                            modifier = Modifier.fillMaxSize(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(
                                8.dp,
                                Alignment.CenterVertically
                            )
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.qr_code_scanner_24px),
                                contentDescription = "Scan QR code icon",
                                modifier = Modifier.size(48.dp)
                            )

                            Text("Scan QR code", style = MaterialTheme.typography.bodyLarge)
                        }
                    }

                    Button(
                        modifier = Modifier
                            .weight(1f)
                            .height(140.dp),
                        shape = MaterialTheme.shapes.large,
                        onClick = onConnectManuallyButtonClicked,
                    ) {
                        Column(
                            modifier = Modifier.fillMaxSize(),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(
                                8.dp,
                                Alignment.CenterVertically
                            )
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.input_24px),
                                contentDescription = "Enter manually icon",
                                modifier = Modifier.size(48.dp)
                            )

                            Text("Enter manually", style = MaterialTheme.typography.bodyLarge)
                        }
                    }
                }
            }

            if (recentServers.isNotEmpty()) {
                HomeSection("RECENT CONNECTIONS") {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        val shown = recentServers.sortedByDescending { it.connectedAt }.take(5)
                        for (recentServer in shown) {
                            RecentConnection(
                                recentServer = recentServer,
                                connectingTo = connectingTo,
                                onConnect = {
                                    onConnectRecent(recentServer.nodeId)
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun HomeSection(title: String, content: @Composable () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surface)
    ) {
        Column {
            SectionHeader(title)

            Box(
                modifier = Modifier
                    .clipToBounds()
            ) {
                Column(
                    modifier = Modifier
                        .padding(8.dp)
                ) {
                    content()
                }
            }
        }
    }
    HorizontalDivider(thickness = 1.dp)
}

@Composable
fun RecentConnection(
    recentServer: RecentServerModel,
    connectingTo: String?,
    onConnect: () -> Unit,
) {
    val daysAgo = (now() - recentServer.connectedAt).toInt().seconds.inWholeDays
    val readableDaysAgo = when (daysAgo) {
        0L -> "today"
        1L -> "1 day ago"
        else -> "$daysAgo days ago"
    }
    val detail = "${shortenNodeId(recentServer.nodeId)}, $readableDaysAgo"

    Card(
        modifier = Modifier.fillMaxWidth(),
        onClick = onConnect,
        enabled = connectingTo == null,
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(8.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(
                modifier = Modifier.weight(1f)
            ) {
                Text(
                    text = recentServer.name,
                    style = MaterialTheme.typography.labelLarge,
                )
                Text(
                    text = detail,
                    style = MaterialTheme.typography.labelSmall,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }

            if (connectingTo == recentServer.nodeId) {
                CircularProgressIndicator(
                    modifier = Modifier.size(24.dp)
                )
            } else {
                Icon(
                    painter = painterResource(Res.drawable.arrow_forward_24px),
                    contentDescription = null
                )
            }
        }
    }
}

@Composable
fun HomeScreenSandbox() {
    val recentServers = remember {
        buildList {
            repeat(10) {
                add(
                    RecentServerModel(
                        nodeId = mockNodeId(),
                        name = "My Desktop",
                        connectedAt = now() - (0uL..1_000_000uL).random()
                    )
                )
            }
        }
    }

    var connectingTo by remember { mutableStateOf<String?>(null) }

    val coroutineScope = rememberCoroutineScope()
    val onConnectRecent = { nodeId: String ->
        coroutineScope.launch {
            connectingTo = nodeId
            delay(500)
            connectingTo = null
        }
        Unit
    }

    HomeScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        recentServers = recentServers,
        connectingTo = connectingTo,
        onPickDownloadDirectory = {},
        onConnectQRButtonClicked = {},
        onConnectManuallyButtonClicked = {},
        onConnectRecent = onConnectRecent,
    )
}

