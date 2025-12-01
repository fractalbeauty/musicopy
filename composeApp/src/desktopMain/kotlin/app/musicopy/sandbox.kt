package app.musicopy

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.graphics.rememberGraphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import androidx.compose.ui.window.rememberWindowState
import app.musicopy.ui.Theme
import app.musicopy.ui.components.Info
import app.musicopy.ui.screens.TransferScreenFinishedSandbox
import app.musicopy.ui.screens.TransferScreenSandbox
import app.musicopy.ui.screenshots.DesktopHeroScreenshot
import app.musicopy.ui.screenshots.MobileHeroScreenshot
import app.musicopy.ui.screenshots.MobileHomeScreenshot
import app.musicopy.ui.screenshots.MobilePreTransferScreenshot
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
//    SandboxScreenshot()
    TransferScreenFinishedSandbox()
}

val DIMENSIONS_MOBILE = 350 to 600
val DIMENSIONS_DESKTOP = 1024 to 768

class ScreenshotConfig(
    val file: String,
    val description: String = "",
    val dimensions: Pair<Int, Int>,
    val density: Float = 1f,
    val content: @Composable () -> Unit,
)

const val initialConfig = 2
val screenshotConfigs = listOf(
    // web hero images
    ScreenshotConfig(
        file = "web/public/static/hero_mobile.png",
        description = "web home hero",
        dimensions = 350 to 550,
        content = { MobileHeroScreenshot() }
    ),
    ScreenshotConfig(
        file = "web/public/static/hero_desktop.png",
        description = "web home hero - expand status jobs",
        dimensions = 900 to 550,
        content = { DesktopHeroScreenshot() }
    ),

    // google - phone
    ScreenshotConfig(
        file = "screenshots/google/phone/google_phone_1.png",
        description = "transfer - expand all. should be 14 transferring",
        dimensions = 1080 to 1920,
        density = 3f,
        content = { MobileTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/google/phone/google_phone_2.png",
        description = "pretransfer - select boneyard, expand fishmonger, select all undownloaded. should be 14 selected",
        dimensions = 1080 to 1920,
        density = 3f,
        content = { MobilePreTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/google/phone/google_phone_3.png",
        dimensions = 1080 to 1920,
        density = 3f,
        content = { MobileHomeScreenshot() }
    ),

    // google - tablet
    ScreenshotConfig(
        file = "screenshots/google/tablet/google_tablet_1.png",
        description = "transfer - expand all. should be 14 transferring",
        dimensions = 1080 to 1920,
        density = 1.5f,
        content = { MobileTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/google/tablet/google_tablet_2.png",
        description = "pretransfer - select boneyard, expand fishmonger, select all undownloaded. should be 14 selected",
        dimensions = 1080 to 1920,
        density = 1.5f,
        content = { MobilePreTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/google/tablet/google_tablet_3.png",
        dimensions = 1080 to 1920,
        density = 1.5f,
        content = { MobileHomeScreenshot() }
    ),

    // apple - phone 6.5"
    ScreenshotConfig(
        file = "screenshots/apple/phone65/apple_phone65_1.png",
        description = "transfer - expand all. should be 14 transferring",
        dimensions = 1284 to 2778,
        density = 3.5f,
        content = { MobileTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/apple/phone65/apple_phone65_2.png",
        description = "pretransfer - select boneyard, expand fishmonger, select all undownloaded. should be 14 selected",
        dimensions = 1284 to 2778,
        density = 3.5f,
        content = { MobilePreTransferScreenshot() }
    ),
    ScreenshotConfig(
        file = "screenshots/apple/phone65/apple_phone65_3.png",
        dimensions = 1284 to 2778,
        density = 3.5f,
        content = { MobileHomeScreenshot() }
    ),
)

@Composable
fun SandboxScreenshot() {
    var configIndex by remember { mutableStateOf(initialConfig) }
    val config = screenshotConfigs.getOrNull(configIndex)

    config?.let { config ->
        val width = config.dimensions.first
        val height = config.dimensions.second

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            Info {
                Text(
                    text = "Config ${configIndex + 1} / ${screenshotConfigs.size}",
                    style = MaterialTheme.typography.bodyMedium
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
                density = config.density,
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
    density: Float,
    content: @Composable () -> Unit,
) {
    val coroutineScope = rememberCoroutineScope()
    val graphicsLayer = rememberGraphicsLayer()

    Info {
        Text(
            text = "Image size: $width x $height",
            style = MaterialTheme.typography.bodyMedium
        )
        Text(
            text = "Device size: ${width / density} x ${height / density} (density = $density)",
            style = MaterialTheme.typography.bodyMedium
        )

        OutlinedButton(
            onClick = {
                coroutineScope.launch {
                    val bitmap = graphicsLayer.toImageBitmap()
                    val bytes = bitmap.toByteArray()

                    val file = File("../${file}")

                    val parent = File(file.parent)
                    parent.mkdirs()

                    file.writeBytes(bytes)
                }
            }
        ) {
            Text("Screenshot")
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .horizontalScroll(rememberScrollState())
    ) {
        CompositionLocalProvider(
            LocalDensity provides Density(density)
        ) {
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
                        .size(width = width.dp / density, height = height.dp / density)
                        .background(MaterialTheme.colorScheme.surface)
                ) {
                    content()
                }
            }
        }
    }
}
