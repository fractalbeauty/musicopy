package app.musicopy.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun SectionCard(
    title: String,
    body: @Composable () -> Unit,
    onCancel: (() -> Unit)? = null,
    onAction: (() -> Unit)? = null,
    actionLabel: String = "Submit",
    actionEnabled: Boolean = true,
    actionLoading: Boolean? = null,
) {
    val hasAction = onCancel !== null || onAction !== null

    Card(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(title, style = MaterialTheme.typography.titleLarge)

            body()

            if (hasAction) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp, alignment = Alignment.End),
                ) {
                    onCancel?.let {
                        OutlinedButton(
                            onClick = onCancel,
                        ) {
                            Text("Cancel")
                        }
                    }
                    onAction?.let {
                        LoadingButton(
                            onClick = onAction,
                            label = actionLabel,
                            enabled = actionEnabled,
                            loading = actionLoading,
                        )
                    }
                }
            }
        }
    }
}
