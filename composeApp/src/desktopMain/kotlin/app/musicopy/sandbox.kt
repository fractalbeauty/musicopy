package app.musicopy

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import app.musicopy.ui.Theme
import app.musicopy.ui.components.Info
import app.musicopy.ui.screenshots.DesktopHomeScreenshot
import app.musicopy.ui.screenshots.MobileTransferScreenshot
import com.composeunstyled.Text
import io.github.alexzhirkevich.qrose.toByteArray
import kotlinx.coroutines.launch
import java.io.File

fun main() = application {
    // use mock settings store
    AppSettings.installMockSettings()

    val state = rememberWindowState(
        size = DpSize(WINDOW_WIDTH.dp, WINDOW_HEIGHT.dp),
    )

    Window(
        title = "Musicopy [Sandbox]",
        onCloseRequest = ::exitApplication,
        state = state,
    ) {
        val platformAppContext = PlatformAppContext()
        val platformActivityContext = PlatformActivityContext(mainWindow = window)

        Sandbox()

        // TODO
        Box(modifier = Modifier.offset(x = 8.dp, y = 8.dp)) {
            Text("window: ${LocalWindowInfo.current.containerSize}")
        }
    }
}

@Composable
private fun Sandbox() {
    Theme {
        SandboxContent()
    }
}

@Composable
private fun SandboxContent() {
    SandboxScreenshot()
}

val DIMENSIONS_MOBILE = 350 to 600
val DIMENSIONS_DESKTOP = 1024 to 768

class ScreenshotConfig(
    val file: String,
    val description: String,
    val dimensions: Pair<Int, Int>,
    val content: @Composable () -> Unit,
)

const val initialConfig = 1
val screenshotConfigs = listOf(
    ScreenshotConfig(
        file = "web/public/static/hero_mobile.png",
        description = "web home hero",
        dimensions = 350 to 550,
        content = { MobileTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "web/public/static/hero_desktop.png",
        description = "web home hero",
        dimensions = 900 to 550,
        content = { DesktopHomeScreenshot() }
    )
)

@Composable
fun SandboxScreenshot() {
    var configIndex by remember { mutableStateOf(initialConfig) }
    val config = screenshotConfigs.getOrNull(configIndex)

    config?.let { config ->
        val width = config.dimensions.first
        val height = config.dimensions.second

        Column(
            modifier = Modifier.fillMaxSize().padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Info {
                Text(
                    text = "Config ${configIndex + 1} / ${screenshotConfigs.size}"
                )

                Text(
                    text = "File: ${config.file}",
                    style = MaterialTheme.typography.bodyMedium
                )

                Text(
                    text = "Description: ${config.description}",
                    style = MaterialTheme.typography.bodyMedium
                )

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(
                        enabled = configIndex > 0,
                        onClick = { configIndex -= 1 },
                    ) {
                        Text("Prev")
                    }

                    OutlinedButton(
                        enabled = configIndex < screenshotConfigs.size - 1,
                        onClick = { configIndex += 1 },
                    ) {
                        Text("Next")
                    }
                }
            }

            Screenshot(
                file = config.file,
                width = width,
                height = height,
            ) {
                config.content()
            }
        }
    }
}

@Composable
private fun Screenshot(
    file: String,
    width: Int,
    height: Int,
    content: @Composable () -> Unit,
) {
    val coroutineScope = rememberCoroutineScope()
    val graphicsLayer = rememberGraphicsLayer()

    Info {
        Text(
            text = "Screenshot size: $width x $height",
            style = MaterialTheme.typography.bodyMedium
        )

        OutlinedButton(
            onClick = {
                coroutineScope.launch {
                    val bitmap = graphicsLayer.toImageBitmap()
                    val bytes = bitmap.toByteArray()
                    File("../${file}").writeBytes(bytes)
                }
            }
        ) {
            Text("Screenshot")
        }
    }

    Box(
        modifier = Modifier
            .drawWithContent {
                graphicsLayer.record {
                    this@drawWithContent.drawContent()
                }
                drawLayer(graphicsLayer)
            }
    ) {
        Box(
            modifier = Modifier
                .size(width = width.dp, height = height.dp)
                .background(MaterialTheme.colorScheme.surface)
        ) {
            content()
        }
    }
}
