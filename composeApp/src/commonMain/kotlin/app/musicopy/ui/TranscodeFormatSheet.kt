package app.musicopy.ui

import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import app.musicopy.AppSettings
import com.composables.core.DragIndication
import com.composables.core.ModalBottomSheet
import com.composables.core.ModalBottomSheetState
import com.composables.core.Scrim
import com.composables.core.Sheet
import com.composables.core.SheetDetent.Companion.FullyExpanded
import com.composables.core.SheetDetent.Companion.Hidden
import com.composables.core.rememberModalBottomSheetState

class TranscodeFormatSheetState(
    internal val inner: ModalBottomSheetState,
) {
    fun peek() {
        inner.targetDetent = Peek
    }

    fun hide() {
        inner.targetDetent = Hidden
    }
}

@Composable
fun rememberTranscodeFormatSheetState(): TranscodeFormatSheetState {
    val inner = rememberModalBottomSheetState(
        initialDetent = Hidden,
        detents = listOf(Hidden, Peek, FullyExpanded)
    )
    return TranscodeFormatSheetState(
        inner,
    )
}

@Composable
fun TranscodeFormatSheet(appSettings: AppSettings, state: TranscodeFormatSheetState) {
    ModalBottomSheet(state = state.inner) {
        Scrim(
            enter = fadeIn(),
            exit = fadeOut()
        )

        Sheet(
            modifier = Modifier
                .shadow(4.dp, RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
                .clip(RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
                .background(MaterialTheme.colorScheme.surfaceContainer)
                .widthIn(max = 640.dp)
                .fillMaxWidth()
                .imePadding()
        ) {
            Column {
                Box(
                    modifier = Modifier.fillMaxWidth(),
                    contentAlignment = Alignment.TopCenter
                ) {
                    DragIndication(
                        modifier = Modifier
                            .padding(top = 8.dp)
                            .background(
                                MaterialTheme.colorScheme.outline,
                                RoundedCornerShape(100)
                            )
                            .width(32.dp)
                            .height(4.dp)
                    )
                }

                Column(
                    modifier = Modifier.padding(8.dp).padding(bottom = 20.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    val onSetFormat = { transcodeFormatId: String ->
                        appSettings.transcodeFormat = transcodeFormatId
                        state.hide()
                    }

                    TranscodeFormatButton(TranscodeFormat.Opus128, onSetFormat)
                    TranscodeFormatButton(TranscodeFormat.Opus64, onSetFormat)
                    TranscodeFormatButton(TranscodeFormat.Mp3v0, onSetFormat)
                    TranscodeFormatButton(TranscodeFormat.Mp3v5, onSetFormat)
                    TranscodeFormatButton(TranscodeFormat.None, onSetFormat)
                }
            }
        }
    }
}

@Composable
internal fun TranscodeFormatButton(
    transcodeFormat: TranscodeFormat,
    onSetFormat: (id: String) -> Unit,
) {
    val label = buildString {
        append(transcodeFormat.label)
        if (transcodeFormat.formatLabel != null) {
            append(": ")
            append(transcodeFormat.formatLabel)
        }
    }

    OutlinedCard(
        modifier = Modifier.fillMaxWidth(),
        onClick = {
            onSetFormat(transcodeFormat.id)
        },
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(8.dp)
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Bold
            )
            Text(
                text = transcodeFormat.description,
                style = MaterialTheme.typography.bodyLarge,
            )
        }
    }
}

enum class TranscodeFormat(
    val id: String,
    val label: String,
    val formatLabel: String?,
    val description: String,
) {
    Opus128(
        "opus128",
        "Best Quality",
        "Opus 128kb/s",
        "Optimized for quality, ~300 songs per GB."
    ),
    Opus64(
        "opus64",
        "Best Size",
        "Opus 64kb/s",
        "Optimized for size, ~600 songs per GB."
    ),
    Mp3v0(
        "mp3v0",
        "Compatibility + Quality",
        "MP3 V0",
        "Use with apps that don't support Opus.\nOptimized for quality, ~150 songs per GB."
    ),
    Mp3v5(
        "mp3v5",
        "Compatibility + Size",
        "MP3 V5",
        "Use with apps that don't support Opus.\nOptimized for size, ~300 songs per GB."
    ),
    None(
        "none",
        "Original",
        null,
        "Don't convert files when transferring."
    );

    companion object
}

fun TranscodeFormat.Companion.fromId(id: String): TranscodeFormat? {
    return when (id) {
        TranscodeFormat.Opus128.id -> TranscodeFormat.Opus128
        TranscodeFormat.Opus64.id -> TranscodeFormat.Opus64
        TranscodeFormat.Mp3v0.id -> TranscodeFormat.Mp3v0
        TranscodeFormat.Mp3v5.id -> TranscodeFormat.Mp3v5
        TranscodeFormat.None.id -> TranscodeFormat.None
        else -> null
    }
}
