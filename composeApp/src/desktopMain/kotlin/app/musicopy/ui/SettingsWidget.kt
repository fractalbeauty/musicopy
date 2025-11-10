package app.musicopy.ui

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.formatSize
import app.musicopy.openDirectoryInExplorer
import app.musicopy.rememberPoll
import app.musicopy.ui.components.WidgetContainer
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.delete_sweep_24px
import musicopy_root.musicopy.generated.resources.folder_open_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.TranscodePolicy

@Composable
fun SettingsWidget(
    libraryModel: LibraryModel,
    onSetTranscodePolicy: (TranscodePolicy) -> Unit,
) {
    WidgetContainer(
        title = "OPTIONS",
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Card(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier.padding(4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
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
            }

            Card(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier.padding(4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
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
                            // TODO
                        },
                    ) {
                        Icon(
                            painter = painterResource(Res.drawable.delete_sweep_24px),
                            contentDescription = "Clean button"
                        )
                    }
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
