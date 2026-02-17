package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.ui.QRScanner
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.TopBar
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.open_in_new_24px
import org.jetbrains.compose.resources.painterResource

@Composable
fun ConnectQRScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,

    autoLaunch: Boolean,
    isConnecting: Boolean,
    onSubmit: (String) -> Unit,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Scan QR code",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Info {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Scan the QR code in the connect widget on the desktop app.",
                        style = MaterialTheme.typography.bodyMedium
                    )

                    Text(
                        "You can also connect manually by entering a code.",
                        style = MaterialTheme.typography.bodyMedium
                    )

                    val uriHandler = LocalUriHandler.current

                    OutlinedCard(
                        modifier = Modifier.fillMaxWidth(),
                        onClick = {
                            uriHandler.openUri("https://musicopy.app/download")
                        },
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
                                    text = "Download desktop app",
                                    style = MaterialTheme.typography.labelLarge,
                                )
                                Text(
                                    text = "musicopy.app/download",
                                    style = MaterialTheme.typography.labelSmall,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis
                                )
                            }

                            Icon(
                                painter = painterResource(Res.drawable.open_in_new_24px),
                                contentDescription = null
                            )
                        }
                    }
                }
            }

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.Center) {
                QRScanner(
                    autoLaunch = autoLaunch,
                    onResult = { nodeId ->
                        onSubmit(nodeId)
                    }
                )
            }
        }
    }
}
