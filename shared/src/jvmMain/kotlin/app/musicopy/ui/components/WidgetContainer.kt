package app.musicopy.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.unit.dp
import app.musicopy.ui.widgetHeadline

@Composable
fun WidgetContainer(
    modifier: Modifier = Modifier,
    title: String,
    content: @Composable () -> Unit,
) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surface)
                .border(BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant))
                .then(modifier),
    ) {
        Column {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(MaterialTheme.colorScheme.primaryContainer),
            ) {
                Box(
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .padding(8.dp),
                ) {
                    Text(
                        title,
                        style = MaterialTheme.typography.widgetHeadline,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                    )
                }
            }

            Box(
                modifier =
                    Modifier
                        .clipToBounds(),
            ) {
                Column(
                    modifier =
                        Modifier
                            .padding(8.dp),
                ) {
                    content()
                }
            }
        }
    }
}
