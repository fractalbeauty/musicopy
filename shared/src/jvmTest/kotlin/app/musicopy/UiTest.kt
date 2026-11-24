package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asSkiaBitmap
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.platform.WindowInfo
import androidx.compose.ui.test.ComposeUiTest
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.runComposeUiTest
import io.kotest.core.test.TestCase
import io.kotest.core.test.TestScope
import org.jetbrains.skia.EncodedImageFormat
import org.jetbrains.skia.Image
import java.io.File

/**
 * Runs a Compose UI test inside a Kotest test.
 *
 * If the test fails, a screenshot of the UI is saved to `build/test-results/screenshots/<test name>.failure.png`.
 * Use [captureScreenshot] to save screenshots at other points.
 */
@OptIn(ExperimentalTestApi::class)
fun TestScope.uiTest(block: ComposeUiTest.() -> Unit) {
    val name = testCase.screenshotName()

    runComposeUiTest {
        currentUiTest.set(UiTestContext(name))

        try {
            block()
        } catch (t: Throwable) {
            saveScreenshot(onRoot().captureToImage(), "$name.failure")
            throw t
        } finally {
            currentUiTest.remove()
        }
    }
}

/**
 * Sets the content with the providers that tests need.
 */
@OptIn(ExperimentalTestApi::class)
fun ComposeUiTest.setContentWithTestProviders(content: @Composable () -> Unit) {
    setContent {
        TestWindowInfoProvider(content)
    }
}

/**
 * Makes the window count as focused. Offscreen test windows aren't focused, which changes how
 * focus-dependent UI renders.
 */
@Composable
fun TestWindowInfoProvider(content: @Composable () -> Unit) {
    val windowInfo = remember { TestWindowInfo() }
    CompositionLocalProvider(LocalWindowInfo provides windowInfo) {
        content()
    }
}

/**
 * Saves a screenshot of the UI if `MUSICOPY_UI_TEST_SCREENSHOTS` is set. Must be called within [uiTest].
 *
 * Saved to `build/test-results/screenshots/<test name>.<index>.<label>.png`.
 */
@OptIn(ExperimentalTestApi::class)
fun ComposeUiTest.captureScreenshot(label: String) {
    val context = currentUiTest.get() ?: error("captureScreenshot must be called within uiTest { }")
    if (System.getenv(SCREENSHOTS_ENV_VAR) == null) return
    saveScreenshot(onRoot().captureToImage(), "${context.name}.${context.screenshotCount++}.$label")
}

internal class TestWindowInfo : WindowInfo {
    override val isWindowFocused = true
    override var keyboardModifiers: PointerKeyboardModifiers by mutableStateOf(PointerKeyboardModifiers())
}

private val currentUiTest = ThreadLocal<UiTestContext>()

private class UiTestContext(
    val name: String,
) {
    var screenshotCount = 0
}

/**
 * Whether to save [captureScreenshot] screenshots when running UI tests.
 */
private const val SCREENSHOTS_ENV_VAR = "MUSICOPY_UI_TEST_SCREENSHOTS"

/**
 * A file-name-safe name for the test, like `ConnectManuallyScreenTest.empty_code.disables_connect`.
 */
private fun TestCase.screenshotName(): String {
    val path = generateSequence(this) { it.parent }.map { it.name.name }.toList().reversed()
    val specName = spec::class.simpleName ?: "Spec"
    return "$specName.${path.joinToString(".")}".replace(Regex("[^A-Za-z0-9_.-]+"), "_")
}

internal fun saveScreenshot(
    bitmap: ImageBitmap,
    name: String,
    directory: String = "build/test-results/screenshots",
) {
    val bytes =
        Image
            .makeFromBitmap(bitmap.asSkiaBitmap())
            .encodeToData(EncodedImageFormat.PNG)!!
            .bytes
    val outFile = File("$directory/$name.png")
    outFile.parentFile.mkdirs()
    outFile.writeBytes(bytes)
    println("Saved screenshot to ${outFile.absolutePath}")
}
