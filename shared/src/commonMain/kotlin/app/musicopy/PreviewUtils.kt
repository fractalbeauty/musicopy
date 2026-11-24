package app.musicopy

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import app.musicopy.ui.Theme

/**
 * Wraps a `@Preview` in the app theme with the theme's background.
 */
@Composable
fun PreviewWrapper(
    padded: Boolean = false,
    content: @Composable () -> Unit,
) {
    Theme {
        Box(
            Modifier
                .background(MaterialTheme.colorScheme.background)
                .letIf(padded) { it.padding(8.dp) },
        ) {
            content()
        }
    }
}

/**
 * Sizes a screen preview like a phone.
 */
@Composable
fun ScreenPreview(content: @Composable () -> Unit) {
    Box(Modifier.size(width = 360.dp, height = 720.dp)) {
        content()
    }
}
