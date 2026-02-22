package app.musicopy

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.awaitApplication
import androidx.compose.ui.window.rememberWindowState
import com.composeunstyled.Text
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val platformAppContext = PlatformAppContext()
    val appSettings = AppSettings()

    // TODO: measure how long blocking on this takes
    val coreInstance = CoreInstance.start(platformAppContext, appSettings)

    awaitApplication {
        val state = rememberWindowState()

        Window(
            title = "Musicopy",
            onCloseRequest = ::exitApplication,
            state = state
        ) {
            val platformActivityContext = PlatformActivityContext(mainWindow = window)

            App(
                platformAppContext = platformAppContext,
                platformActivityContext = platformActivityContext,
                coreInstance = coreInstance,
                appSettings = appSettings
            )

            // TODO
            Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
                Text("window: ${LocalWindowInfo.current.containerSize}")
            }
        }
    }
}
