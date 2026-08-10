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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.ui.components.Info
import app.musicopy.ui.components.LoadingButton
import app.musicopy.ui.components.TopBar
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.open_in_new_24px
import org.jetbrains.compose.resources.painterResource

@Composable
fun ConnectManuallyScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,
    isConnecting: Boolean,
    onSubmit: (String) -> Unit,
    onCancel: () -> Unit,
) {
    var value by remember { mutableStateOf("") }

    val trimmedValue = value.replace("\\s+".toRegex(), "")

    val isEmpty = trimmedValue.isEmpty()
    val isValid = trimmedValue.length == 64
    val isError = !isEmpty && !isValid

    val supportingText =
        if (isError) {
            @Composable {
                Text("Invalid code.")
            }
        } else {
            null
        }

    Scaffold(
        topBar = {
            TopBar(
                title = "Connect manually",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel,
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Info {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        "Click \"enter manually\" in the connect widget on the desktop app and enter the code below.",
                        style = MaterialTheme.typography.bodyMedium,
                    )

                    Text(
                        "You can also scan the QR code to connect automatically.",
                        style = MaterialTheme.typography.bodyMedium,
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
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(
                                    text = "Download desktop app",
                                    style = MaterialTheme.typography.labelLarge,
                                )
                                Text(
                                    text = "musicopy.app/download",
                                    style = MaterialTheme.typography.labelSmall,
                                    maxLines = 1,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }

                            Icon(
                                painter = painterResource(Res.drawable.open_in_new_24px),
                                contentDescription = null,
                            )
                        }
                    }
                }
            }

            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                label = {
                    Text("Code")
                },
                maxLines = 1,
                modifier = Modifier.fillMaxWidth(),
                isError = isError,
                supportingText = supportingText,
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                LoadingButton(
                    label = "Connect",
                    onClick = { onSubmit(trimmedValue) },
                    enabled = isValid,
                    loading = isConnecting,
                )
            }
        }
    }
}

@Composable
fun ConnectManuallyScreenSandbox() {
    var isConnecting by remember { mutableStateOf(false) }

    ConnectManuallyScreen(
        onShowNodeStatus = {},
        snackbarHost = {},
        isConnecting = isConnecting,
        onSubmit = { isConnecting = true },
        onCancel = { isConnecting = false },
    )
}
