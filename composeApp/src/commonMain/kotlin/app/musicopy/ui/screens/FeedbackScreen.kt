package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.dp
import app.musicopy.ui.components.LoadingButton
import app.musicopy.ui.components.TopBar
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun FeedbackScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,
    isSending: Boolean,
    onSubmit: (description: String) -> Unit,
    onCancel: () -> Unit,
) {
    val focusManager = LocalFocusManager.current

    var description by remember { mutableStateOf("") }

    var isSubmitted by remember { mutableStateOf(false) }

    val scope = rememberCoroutineScope()
    val handleSubmit = {
        focusManager.clearFocus()

        onSubmit(description)

        scope.launch {
            delay(1000)
            isSubmitted = true
        }
    }

    Scaffold(
        topBar = {
            TopBar(
                title = "Send feedback",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding).padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            OutlinedTextField(
                value = description,
                onValueChange = { description = it },
                label = { Text("Feedback") },
                minLines = 5,
                maxLines = 10,
                modifier = Modifier.fillMaxWidth(),
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End
            ) {
                if (!isSubmitted) {
                    LoadingButton(
                        label = "Send",
                        onClick = { handleSubmit() },
                        enabled = description.isNotBlank(),
                        loading = isSending,
                    )
                } else {
                    LoadingButton(
                        label = "Done",
                        onClick = onCancel,
                    )
                }
            }
        }
    }
}

@Composable
fun FeedbackScreenSandbox() {
    var isSending by remember { mutableStateOf(false) }

    FeedbackScreen(
        snackbarHost = {},
        onShowNodeStatus = {},
        isSending = isSending,
        onSubmit = { isSending = true },
        onCancel = { isSending = false },
    )
}
