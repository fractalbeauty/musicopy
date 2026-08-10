package app.musicopy.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CutCornerShape
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun Info(content: @Composable () -> Unit) {
    val shape = CutCornerShape(topEnd = 16.dp)

    Box(
        Modifier
            .fillMaxWidth()
            .background(
                color = MaterialTheme.colorScheme.secondary,
            ),
    ) {
        Box(
            Modifier
                .fillMaxWidth()
                .background(
                    color = MaterialTheme.colorScheme.secondaryContainer,
                    shape = shape,
                ).border(
                    color = MaterialTheme.colorScheme.outlineVariant,
                    width = 1.dp,
                    shape = shape,
                ),
        ) {
            Column(Modifier.padding(8.dp)) {
                CompositionLocalProvider(LocalContentColor provides MaterialTheme.colorScheme.onSecondaryContainer) {
                    content()
                }
            }
        }
    }
}
