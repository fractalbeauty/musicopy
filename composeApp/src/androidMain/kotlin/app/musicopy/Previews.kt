package app.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.musicopy.ui.screens.ConnectQRScreen

@Composable
fun ScreenPreview(
    content: @Composable() () -> Unit,
) {
    MaterialTheme {
        Scaffold { innerPadding ->
            Box(modifier = Modifier.padding(innerPadding)) {
                content()
            }
        }
    }
}

@Preview(showSystemUi = true)
@Composable
fun ConnectQRScreenPreview() {
    ScreenPreview {
        ConnectQRScreen(
            snackbarHost = {},
            onShowNodeStatus = {},

            autoLaunch = false,
            onSubmit = {},
            onCancel = {},
            isConnecting = false
        )
    }
}
