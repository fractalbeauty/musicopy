package app.musicopy.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TriStateCheckbox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.unit.dp
import app.musicopy.AppSettings
import app.musicopy.ui.components.SectionHeader
import app.musicopy.ui.components.TopBar
import app.musicopy.ui.components.aboutText
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_forward_24px
import org.jetbrains.compose.resources.painterResource
import kotlin.time.ExperimentalTime

@OptIn(ExperimentalTime::class)
@Composable
fun SettingsScreen(
    appSettings: AppSettings,

    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,

    onShowFeedback: () -> Unit,
    onClearData: () -> Unit,
    onCancel: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopBar(
                title = "Settings",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
        ) {
            SectionHeader("ABOUT")

            Box(
                modifier = Modifier.padding(16.dp)
            ) {
                Text(
                    text = aboutText(
                        supportText = false
                    ),
                    style = MaterialTheme.typography.bodyMedium
                )
            }

            HorizontalDivider(thickness = 1.dp)

            SectionHeader("SUPPORT")

            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedCard(
                    modifier = Modifier.fillMaxWidth(),
                    onClick = onShowFeedback,
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "Send feedback with logs",
                            style = MaterialTheme.typography.bodyMedium
                        )

                        Icon(
                            painter = painterResource(Res.drawable.arrow_forward_24px),
                            contentDescription = null
                        )
                    }
                }
            }

            HorizontalDivider(thickness = 1.dp)

            SectionHeader("ADVANCED")

            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        "Clear data",
                        style = MaterialTheme.typography.bodyMedium
                    )

                    Button(
                        onClick = onClearData,
                    ) {
                        Text("Clear")
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Text(
                        text = "Debug mode",
                        style = MaterialTheme.typography.bodyMedium,
                    )

                    val detailedErrors by appSettings.detailedErrorsFlow.collectAsState(appSettings.detailedErrors)
                    TriStateCheckbox(
                        state = ToggleableState(detailedErrors),
                        onClick = {
                            appSettings.detailedErrors = !appSettings.detailedErrors
                        }
                    )
                }
            }

            HorizontalDivider(thickness = 1.dp)
        }
    }
}

@Composable
fun SettingsScreenSandbox() {
    val appSettings = remember { AppSettings.createMock() }

    SettingsScreen(
        appSettings = appSettings,

        snackbarHost = {},
        onShowNodeStatus = {},

        onShowFeedback = {},
        onClearData = {},
        onCancel = {},
    )
}
