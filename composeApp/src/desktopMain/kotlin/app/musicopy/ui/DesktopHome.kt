package app.musicopy.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import androidx.compose.ui.unit.dp
import com.composables.core.Dialog
import com.composables.core.DialogPanel
import com.composables.core.DialogState
import com.composables.core.Scrim
import com.composables.core.rememberDialogState
import kotlinx.datetime.Instant
import kotlinx.datetime.format
import kotlinx.datetime.format.DateTimeComponents
import kotlinx.datetime.format.MonthNames
import kotlinx.datetime.format.char
import musicopy_root.musicopy.BuildConfig
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.info_24px
import musicopy_root.musicopy.generated.resources.icon
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.TranscodePolicy

@Composable
fun DesktopHome(
    libraryModel: LibraryModel,
    nodeModel: NodeModel,
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
) {
    val oneCol = LocalWindowInfo.current.containerSize.width < 600

    val aboutState = rememberDialogState(initiallyVisible = false)
    AboutDialog(
        state = aboutState,
        onClose = {
            aboutState.visible = false
        }
    )

    Column(
        modifier = Modifier.fillMaxWidth().padding(8.dp)
    ) {
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
                onClick = {
                    aboutState.visible = true
                },
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
                onSetTranscodePolicy = onSetTranscodePolicy,
                onDeleteUnusedTranscodes = onDeleteUnusedTranscodes,
                onDeleteAllTranscodes = onDeleteAllTranscodes,
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

@Composable
private fun AboutDialog(
    state: DialogState,
    onClose: () -> Unit,
) {
    Dialog(state = state, onDismiss = onClose) {
        Scrim()
        DialogPanel(
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
                        text = buildAnnotatedString {
                            withUrl(
                                "https://musicopy.app/manual",
                            ) {
                                append("User Manual")
                            }
                            append("  ⋅  ")
                            withUrl(
                                "https://github.com/fractalbeauty/musicopy",
                            ) {
                                append("Source")
                            }
                            appendLine()

                            appendLine()

                            val buildTime = Instant.fromEpochMilliseconds(BuildConfig.BUILD_TIME)
                            val buildDate = buildTime.format(DateTimeComponents.Format {
                                monthName(MonthNames.ENGLISH_FULL)
                                char(' ')
                                dayOfMonth()
                                chars(", ")
                                year()
                            })
                            appendLine("Version ${BuildConfig.APP_VERSION}, built on $buildDate.")

                            appendLine()

                            append(
                                "Musicopy is available under the "
                            )
                            withUrl(
                                "https://github.com/fractalbeauty/musicopy/blob/main/LICENSE",
                            ) {
                                append("GNU AGPL, version 3")
                            }
                            appendLine(".")

                            appendLine()

                            append("For more information, visit ")
                            withUrl("https://musicopy.app") {
                                append("musicopy.app")
                            }
                            appendLine(
                                "."
                            )

                            append("For support, email ")
                            withUrl("mailto:support@musicopy.app") {
                                append("support@musicopy.app")
                            }
                            appendLine(".")
                        },
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

@Composable
internal fun AnnotatedString.Builder.withUrl(
    url: String,
    content: AnnotatedString.Builder.() -> Unit,
) {
    withLink(
        LinkAnnotation.Url(
            url = url,
            styles = TextLinkStyles(
                style = SpanStyle(
                    color = MaterialTheme.colorScheme.primary,
                    textDecoration = TextDecoration.Underline
                )
            )
        )
    ) {
        content()
    }
}
