package app.musicopy.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.ui.widgetHeadline

@Composable
fun SectionHeader(
    text: String,
    leftContent: (@Composable () -> Unit)? = null,
    contentPadding: PaddingValues = PaddingValues(8.dp),
) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.primaryContainer),
    ) {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(contentPadding),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            leftContent?.invoke()

            Text(
                text = text,
                style = MaterialTheme.typography.widgetHeadline,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
            )
        }
    }
}
