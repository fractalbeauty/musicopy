package app.musicopy.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.musicopy.AppSettings
import app.musicopy.ui.components.aboutText
import app.musicopy.ui.components.withUrl
import com.composeunstyled.UnstyledDialog
import com.composeunstyled.UnstyledDialogPanel
import com.composeunstyled.DialogState
import com.composeunstyled.UnstyledScrim
import com.composeunstyled.rememberDialogState
import com.composeunstyled.DialogProperties
import kotlinx.coroutines.delay
import kotlinx.datetime.LocalDateTime
import kotlinx.datetime.TimeZone
import kotlinx.datetime.format
import kotlinx.datetime.format.DateTimeComponents
import kotlinx.datetime.format.MonthNames
import kotlinx.datetime.format.char
import kotlinx.datetime.toLocalDateTime
import musicopy_root.musicopy.BuildConfig
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.favorite_24px
import musicopy_root.musicopy.generated.resources.heart_plus_24px
import musicopy_root.musicopy.generated.resources.info_24px
import musicopy_root.musicopy.generated.resources.icon
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.StatsModel
import uniffi.musicopy.TranscodePolicy
import uniffi.musicopy.validateLicense
import java.awt.Desktop
import java.net.URI
import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlin.time.Instant

@Composable
fun DesktopHome(
    appSettings: AppSettings,
    libraryModel: LibraryModel,
    nodeModel: NodeModel,
    statsModel: StatsModel,
    showHints: Boolean,
    onAcceptAndTrust: (remoteNodeId: String) -> Unit,
    onAcceptOnce: (remoteNodeId: String) -> Unit,
    onDeny: (remoteNodeId: String) -> Unit,
    onAddLibraryRoot: (name: String, path: String) -> Unit,
    onRemoveLibraryRoot: (name: String) -> Unit,
    onRescanLibrary: () -> Unit,
    onSetTranscodePolicy: (TranscodePolicy) -> Unit,
    onDeleteUnusedTranscodes: () -> Unit,
    onDeleteAllTranscodes: () -> Unit,
    onUntrustNode: (nodeId: String) -> Unit,

    screenshotHideTopBar: Boolean = false,
) {
    val oneCol = LocalWindowInfo.current.containerSize.width < 600

    val aboutState = rememberDialogState(initiallyVisible = false)
    AboutDialog(
        state = aboutState,
        onClose = { aboutState.visible = false }
    )

    val licenseKey by appSettings.licenseKeyFlow.collectAsState(appSettings.licenseKey)
    val hasLicense = licenseKey != null

    // Show nag on launch if the user has transferred files before
    val showLicenseNag = remember {
        statsModel.serverFiles > 1uL
    }

    val licenseState = rememberDialogState(initiallyVisible = !hasLicense && showLicenseNag)
    if (hasLicense) {
        LicenseThanksDialog(
            state = licenseState,
            appSettings = appSettings,
            statsModel = statsModel,
            onClose = { licenseState.visible = false }
        )
    } else {
        LicenseNagDialog(
            state = licenseState,
            appSettings = appSettings,
            statsModel = statsModel,
            onClose = { licenseState.visible = false }
        )
    }

    Column(
        modifier = Modifier.fillMaxWidth().padding(8.dp)
    ) {
        if (!screenshotHideTopBar) {
            Row(
                modifier = Modifier.padding(bottom = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier.padding(end = 8.dp)
                ) {
                    Image(
                        painter = painterResource(Res.drawable.icon),
                        contentDescription = "Musicopy logo",
                        modifier = Modifier
                            .size(44.dp)
                            .border(width = 1.dp, color = MaterialTheme.colorScheme.outlineVariant)
                    )
                }

                Text("MUSICOPY", style = MaterialTheme.typography.logotype)

                Box(modifier = Modifier.weight(1f))

                FilledIconButton(
                    onClick = { licenseState.visible = true },
                    colors = IconButtonDefaults.filledIconButtonColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                ) {
                    Icon(
                        painter = painterResource(
                            if (hasLicense) Res.drawable.favorite_24px
                            else Res.drawable.heart_plus_24px
                        ),
                        contentDescription = "License button icon",
                        modifier = Modifier.size(20.dp)
                    )
                }

                FilledIconButton(
                    onClick = { aboutState.visible = true },
                    colors = IconButtonDefaults.filledIconButtonColors(
                        containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
                        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                ) {
                    Icon(
                        painter = painterResource(Res.drawable.info_24px),
                        contentDescription = "About button icon",
                        modifier = Modifier.size(20.dp)
                    )
                }
            }
        }

        val left = @Composable {
            LibraryWidget(
                libraryModel = libraryModel,
                onAddRoot = onAddLibraryRoot,
                onRemoveRoot = onRemoveLibraryRoot,
                onRescan = onRescanLibrary,

                modifier = Modifier.weight(1f)
            )
            ConnectWidget(
                nodeModel = nodeModel,
                showHints = showHints,
                onAcceptAndTrust = onAcceptAndTrust,
                onAcceptOnce = onAcceptOnce,
                onDeny = onDeny,

                modifier = Modifier.weight(1f)
            )
        }
        val right = @Composable {
            SettingsWidget(
                libraryModel = libraryModel,
                nodeModel = nodeModel,
                onSetTranscodePolicy = onSetTranscodePolicy,
                onDeleteUnusedTranscodes = onDeleteUnusedTranscodes,
                onDeleteAllTranscodes = onDeleteAllTranscodes,
                onUntrustNode = onUntrustNode,
            )
            JobsWidget(
                libraryModel = libraryModel,
                nodeModel = nodeModel,
            )
        }

        if (oneCol) {
            Column(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                left()
                right()
            }
        } else {
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    left()
                }
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    right()
                }
            }
        }
    }
}

@OptIn(ExperimentalTime::class)
@Composable
private fun LicenseNagDialog(
    state: DialogState,
    appSettings: AppSettings,
    statsModel: StatsModel,
    onClose: () -> Unit,
) {
    val windowInfo = LocalWindowInfo.current

    var dismissCountdown by remember { mutableIntStateOf(5) }
    LaunchedEffect(state.visible) {
        if (state.visible) {
            while (dismissCountdown > 0) {
                delay(1000)

                while (!windowInfo.isWindowFocused) {
                    delay(1000)
                }

                dismissCountdown--
            }
        }
    }

    val dismissEnabled = dismissCountdown <= 0

    var isActivating by remember { mutableStateOf(false) }

    var licenseKey by remember { mutableStateOf("") }

    val isEmpty = licenseKey.isEmpty()
    val isValid = validateLicense(licenseKey)
    val isError = !isEmpty && !isValid
    val supportingText = when {
        isEmpty -> ""
        !isValid -> "License key is invalid."
        else -> ""
    }

    UnstyledDialog(
        state = state,
        properties = DialogProperties(
            dismissOnBackPress = dismissEnabled,
            dismissOnClickOutside = dismissEnabled
        )
    ) {
        UnstyledScrim()
        UnstyledDialogPanel(
            modifier = Modifier
                .widthIn(max = 600.dp)
                .padding(16.dp)
        ) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(20.dp)
                ) {
                    Column(
                        modifier = Modifier.fillMaxWidth().padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        if (!isActivating) {
                            Text(
                                text = "Support Musicopy",
                                style = MaterialTheme.typography.headlineSmall,
                            )

                            Text(
                                text = "Musicopy is free to try. If you find it useful, a lifetime license helps support its development.",
                                style = MaterialTheme.typography.bodyMedium,
                            )

                            LicenseDialogStatsText(statsModel)
                        } else {
                            Text(
                                text = "Activate license",
                                style = MaterialTheme.typography.headlineSmall,
                            )

                            Text(
                                text = buildAnnotatedString {
                                    append("Thank you for your support! A license key will be emailed to you after purchase. If you have questions, please email ")
                                    withUrl("mailto:support@musicopy.app") {
                                        append("support@musicopy.app")
                                    }
                                    append(".")
                                },
                                style = MaterialTheme.typography.bodyMedium,
                            )

                            OutlinedTextField(
                                value = licenseKey,
                                onValueChange = { licenseKey = it },
                                label = {
                                    Text("License key")
                                },
                                maxLines = 1,
                                modifier = Modifier.fillMaxWidth(),
                                isError = isError,
                                supportingText = { Text(supportingText) }
                            )
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        if (!isActivating) {
                            TextButton(
                                onClick = {
                                    isActivating = true
                                },
                            ) {
                                Text(
                                    text = "Already purchased?",
                                    style = MaterialTheme.typography.bodySmall
                                )
                            }

                            Box(modifier = Modifier.weight(1f))

                            TextButton(
                                onClick = onClose,
                                enabled = dismissEnabled,
                            ) {
                                Text(
                                    text = if (dismissCountdown > 0) "Continue ($dismissCountdown)" else "Continue",
                                    style = LocalTextStyle.current.copy(
                                        fontFeatureSettings = "tnum"
                                    )
                                )
                            }

                            Button(
                                onClick = {
                                    Desktop.getDesktop().browse(URI("https://musicopy.app/license"))
                                    isActivating = true
                                }
                            ) {
                                Text("Purchase")
                            }
                        } else {
                            Box(modifier = Modifier.weight(1f))

                            TextButton(
                                onClick = { isActivating = false },
                            ) {
                                Text(
                                    text = "Cancel",
                                )
                            }

                            Button(
                                enabled = isValid,
                                onClick = {
                                    appSettings.licenseKey = licenseKey
                                    appSettings.licenseActivatedAt = Clock.System.now().epochSeconds
                                }
                            ) {
                                Text("Activate")
                            }
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalTime::class)
@Composable
private fun LicenseThanksDialog(
    state: DialogState,
    appSettings: AppSettings,
    statsModel: StatsModel,
    onClose: () -> Unit,
) {
    UnstyledDialog(state = state, onDismiss = onClose) {
        UnstyledScrim()
        UnstyledDialogPanel(
            modifier = Modifier
                .widthIn(max = 600.dp)
                .padding(16.dp)
        ) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(32.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Text(
                        text = "Thank you!",
                        style = MaterialTheme.typography.headlineSmall,
                    )

                    val licenseActivatedAt by appSettings.licenseActivatedAtFlow.collectAsState(
                        appSettings.licenseActivatedAt
                    )

                    val licenseActiveText = licenseActivatedAt?.let { licenseActivatedAt ->
                        val activationTime = Instant.fromEpochSeconds(licenseActivatedAt)
                            .toLocalDateTime(TimeZone.currentSystemDefault())
                        val activationDate = activationTime.format(LocalDateTime.Format {
                            monthName(MonthNames.ENGLISH_FULL)
                            char(' ')
                            day()
                            chars(", ")
                            year()
                        })
                        "Your license was activated on $activationDate"
                    } ?: "Your license was activated"

                    Text(
                        text = "${licenseActiveText}. Thank you for your support!",
                        style = MaterialTheme.typography.bodyMedium,
                    )

                    LicenseDialogStatsText(statsModel)

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End,
                    ) {
                        TextButton(onClick = onClose) {
                            Text("Close")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LicenseDialogStatsText(statsModel: StatsModel) {
    // Don't show if nothing has been transferred yet
    if (statsModel.serverFiles == 0uL) {
        return;
    }

    Text(
        text = "You've transferred ${statsModel.serverFiles} ${
            if (statsModel.serverFiles == 1uL) {
                "file"
            } else {
                "files"
            }
        } across ${statsModel.serverSessions} ${
            if (statsModel.serverSessions == 1uL) {
                "session"
            } else {
                "sessions"
            }
        } so far. ♥",
        style = MaterialTheme.typography.bodyMedium,
    )
}

@Composable
private fun AboutDialog(
    state: DialogState,
    onClose: () -> Unit,
) {
    UnstyledDialog(state = state, onDismiss = onClose) {
        UnstyledScrim()
        UnstyledDialogPanel(
            modifier = Modifier
                .widthIn(max = 600.dp)
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
                        text = "About Musicopy",
                        style = MaterialTheme.typography.headlineSmall,
                    )

                    Text(
                        text = aboutText(),
                        style = MaterialTheme.typography.bodyMedium
                    )

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.End
                    ) {
                        TextButton(
                            onClick = onClose,
                        ) {
                            Text("Close")
                        }
                    }
                }
            }
        }
    }
}
