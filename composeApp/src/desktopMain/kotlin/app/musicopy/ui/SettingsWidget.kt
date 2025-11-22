package app.musicopy.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.formatSize
import app.musicopy.now
import app.musicopy.openDirectoryInExplorer
import app.musicopy.rememberPoll
import app.musicopy.shortenNodeId
import app.musicopy.ui.components.WidgetContainer
import com.composables.core.Dialog
import com.composables.core.DialogPanel
import com.composables.core.DialogState
import com.composables.core.Scrim
import com.composables.core.rememberDialogState
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.chevron_forward_24px
import musicopy_root.musicopy.generated.resources.close_24px
import musicopy_root.musicopy.generated.resources.delete_sweep_24px
import musicopy_root.musicopy.generated.resources.folder_open_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.NodeModel
import uniffi.musicopy.TranscodePolicy
import uniffi.musicopy.TrustedNodeModel
import kotlin.time.Duration.Companion.seconds

@Composable
fun SettingsWidget(
    libraryModel: LibraryModel,
    nodeModel: NodeModel,
    onSetTranscodePolicy: (TranscodePolicy) -> Unit,
    onDeleteUnusedTranscodes: () -> Unit,
    onDeleteAllTranscodes: () -> Unit,
    onUntrustNode: (nodeId: String) -> Unit,
) {
    val cleanTranscodesState = rememberDialogState(initiallyVisible = false)
    CleanTranscodesDialog(
        state = cleanTranscodesState,
        onClose = {
            cleanTranscodesState.visible = false
        },
        onDeleteUnusedTranscodes = onDeleteUnusedTranscodes,
        onDeleteAllTranscodes = onDeleteAllTranscodes,
    )

    WidgetContainer(
        title = "OPTIONS",
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            SettingCard(outlined = true) {
                Text(
                    modifier = Modifier.padding(start = 8.dp).weight(1f),
                    text = "Transcode files",
                    style = MaterialTheme.typography.labelLarge,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )

                Row(horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                    TranscodePolicyButton(
                        text = "when requested",
                        onClick = { onSetTranscodePolicy(TranscodePolicy.IF_REQUESTED) },
                        isSelected = libraryModel.transcodePolicy == TranscodePolicy.IF_REQUESTED,
                        startOuter = true,
                        endOuter = false,
                    )
                    TranscodePolicyButton(
                        text = "ahead of time",
                        onClick = { onSetTranscodePolicy(TranscodePolicy.ALWAYS) },
                        isSelected = libraryModel.transcodePolicy == TranscodePolicy.ALWAYS,
                        startOuter = false,
                        endOuter = true,
                    )
                }
            }

            SettingCard(outlined = true) {
                Column(
                    modifier = Modifier.padding(start = 8.dp).weight(1f)
                ) {
                    Text(
                        text = "Transcodes cache",
                        style = MaterialTheme.typography.labelLarge,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    val count by rememberPoll(1000) {
                        libraryModel.transcodeCountQueued.get() +
                                libraryModel.transcodeCountInprogress.get() +
                                libraryModel.transcodeCountReady.get()
                    }
                    Text(
                        text = "$count files, ${
                            formatSize(
                                libraryModel.transcodesDirSize
                            )
                        }",
                        style = MaterialTheme.typography.labelMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }

                IconButton(
                    onClick = {
                        openDirectoryInExplorer(libraryModel.transcodesDir)
                    },
                ) {
                    Icon(
                        painter = painterResource(Res.drawable.folder_open_24px),
                        contentDescription = "Open button"
                    )
                }

                IconButton(
                    onClick = {
                        cleanTranscodesState.visible = true
                    },
                ) {
                    Icon(
                        painter = painterResource(Res.drawable.delete_sweep_24px),
                        contentDescription = "Clean button"
                    )
                }
            }

            if (nodeModel.trustedNodes.isNotEmpty()) {
                ExpandableCard(
                    labelLeft = {
                        Text(
                            modifier = Modifier.padding(start = 8.dp),
                            text = "Trusted devices",
                            style = MaterialTheme.typography.labelLarge,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    },
                    labelRight = {
                        Text(
                            text = "${nodeModel.trustedNodes.size} device${
                                if (nodeModel.trustedNodes.size != 1) {
                                    "s"
                                } else {
                                    ""
                                }
                            }",
                            style = MaterialTheme.typography.labelLarge,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    },
                    body = {
                        Column(
                            verticalArrangement = Arrangement.spacedBy(4.dp)
                        ) {
                            for (trustedNode in nodeModel.trustedNodes) {
                                TrustedNode(
                                    trustedNode = trustedNode,
                                    onUntrust = { onUntrustNode(trustedNode.nodeId) }
                                )
                            }
                        }
                    }
                )
            }
        }
    }
}

@Composable
internal fun SettingCard(
    outlined: Boolean = false,
    content: @Composable RowScope.() -> Unit,
) {
    val inner = @Composable {
        Row(
            modifier = Modifier.padding(4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            content()
        }
    }

    if (outlined) {
        OutlinedCard(
            modifier = Modifier.fillMaxWidth()
        ) {
            inner()
        }
    } else {
        Card(
            modifier = Modifier.fillMaxWidth()
        ) {
            inner()
        }
    }
}

@Composable
internal fun ExpandableCard(
    labelLeft: @Composable () -> Unit = {},
    labelRight: @Composable () -> Unit = {},
    body: (@Composable () -> Unit)? = null,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    val degrees by animateFloatAsState(if (expanded) 90f else 0f)
    OutlinedCard(
        modifier = Modifier.fillMaxWidth().then(modifier),
    ) {
        Column {
            Row(
                modifier = Modifier
                    .heightIn(min = 48.dp)
                    .fillMaxWidth()
                    .clip(MaterialTheme.shapes.medium)
                    .clickable { expanded = !expanded },
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    modifier = Modifier.padding(4.dp).padding(end = 4.dp),
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
                Box(Modifier.fillMaxWidth().padding(8.dp)) {
                    body?.invoke()
                }
            }
        }
    }
}

@Composable
internal fun TranscodePolicyButton(
    text: String,
    onClick: () -> Unit,
    isSelected: Boolean,
    startOuter: Boolean,
    endOuter: Boolean,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()

    val innerRadius = if (isPressed) 4.dp else if (isSelected) 100.dp else 8.dp
    val animInnerRadius by animateDpAsState(
        targetValue = innerRadius,
        animationSpec = spring(
            dampingRatio = 0.9f,
            stiffness = 1400f
        )
    )

    val outerRadius = 100.dp

    val shape = RoundedCornerShape(
        topStart = if (startOuter) outerRadius else animInnerRadius,
        bottomStart = if (startOuter) outerRadius else animInnerRadius,
        topEnd = if (endOuter) outerRadius else animInnerRadius,
        bottomEnd = if (endOuter) outerRadius else animInnerRadius,
    )

    val selectedColors = ButtonDefaults.buttonColors()
    val unselectedColors = ButtonDefaults.buttonColors(
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
        contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
    )

    FilledTonalButton(
        onClick = onClick,
        interactionSource = interactionSource,
        contentPadding = PaddingValues(18.dp, 8.dp),
        colors = if (isSelected) selectedColors else unselectedColors,
        shape = shape,
    ) {
        Text(text = text)
    }
}

@Composable
private fun CleanTranscodesDialog(
    state: DialogState,
    onClose: () -> Unit,
    onDeleteUnusedTranscodes: () -> Unit,
    onDeleteAllTranscodes: () -> Unit,
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
                        text = "Clean transcodes cache",
                        style = MaterialTheme.typography.headlineSmall,
                    )

                    var selected by remember { mutableStateOf("all") }

                    Column(
                        modifier = Modifier.selectableGroup()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .selectable(
                                    selected = (selected == "all"),
                                    onClick = { selected = "all" },
                                    role = Role.RadioButton,
                                )
                                .padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            RadioButton(
                                selected = (selected == "all"),
                                onClick = null
                            )
                            Text(
                                text = "Delete all transcodes",
                                style = MaterialTheme.typography.bodyMedium,
                            )
                        }

                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .selectable(
                                    selected = (selected == "unused"),
                                    onClick = { selected = "unused" },
                                    role = Role.RadioButton,
                                )
                                .padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            RadioButton(
                                selected = (selected == "unused"),
                                onClick = null
                            )
                            Text(
                                text = "Delete only unused transcodes",
                                style = MaterialTheme.typography.bodyMedium,
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
                            Text("Cancel")
                        }

                        TextButton(
                            onClick = {
                                if (selected == "all") {
                                    onDeleteAllTranscodes()
                                } else if (selected == "unused") {
                                    onDeleteUnusedTranscodes()
                                }

                                onClose()
                            }
                        ) {
                            Text("Delete")
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun TrustedNode(
    trustedNode: TrustedNodeModel,
    onUntrust: () -> Unit,
) {
    val readableDaysAgo = trustedNode.connectedAt?.let { connectedAt ->
        val daysAgo = (now() - connectedAt).toInt().seconds.inWholeDays
        when (daysAgo) {
            0L -> "today"
            1L -> "1 day ago"
            else -> "$daysAgo days ago"
        }
    } ?: "never"
    val detail = "${shortenNodeId(trustedNode.nodeId)}, $readableDaysAgo"

    SettingCard {
        Column(
            modifier = Modifier.padding(start = 8.dp).weight(1f)
        ) {
            Text(
                text = trustedNode.name,
                style = MaterialTheme.typography.labelLarge,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = detail,
                style = MaterialTheme.typography.labelMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        IconButton(
            onClick = onUntrust,
        ) {
            Icon(
                painter = painterResource(Res.drawable.close_24px),
                contentDescription = "Remove button"
            )
        }
    }
}
