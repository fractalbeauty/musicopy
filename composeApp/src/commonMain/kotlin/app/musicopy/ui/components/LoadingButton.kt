package app.musicopy.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun LoadingButton(
    onClick: () -> Unit,
    label: String,
    enabled: Boolean = true,
    loading: Boolean? = null,
) {
    Button(
        onClick = onClick,
        contentPadding =
            if (loading != null) {
                PaddingValues(horizontal = 12.dp)
            } else {
                ButtonDefaults.ContentPadding
            },
        enabled = enabled && loading != true,
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            when (loading) {
                true ->
                    CircularProgressIndicator(
                        modifier = Modifier.size(16.dp),
                    )

                false -> Box(modifier = Modifier.size(16.dp))

                null -> Unit
            }
            Text(label)
            when (loading) {
                true, false -> Box(modifier = Modifier.size(16.dp))

                null -> Unit
            }
        }
    }
}
