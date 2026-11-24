package app.musicopy.ui.screens

import androidx.compose.runtime.Composable
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import app.musicopy.captureScreenshot
import app.musicopy.setContentWithTestProviders
import app.musicopy.ui.Theme
import app.musicopy.uiTest
import io.kotest.core.spec.style.FunSpec
import io.kotest.matchers.shouldBe

@OptIn(ExperimentalTestApi::class)
class ConnectManuallyScreenTest :
    FunSpec({
        val validCode = "a".repeat(64)

        test("empty code has no error and cannot connect") {
            uiTest {
                setContentWithTestProviders { screen(isConnecting = false) }

                onNodeWithText("Invalid code.").assertDoesNotExist()
                onNodeWithText("Connect").assertIsNotEnabled()
                captureScreenshot("empty")
            }
        }

        test("short code shows an error and cannot connect") {
            uiTest {
                setContentWithTestProviders { screen(isConnecting = false) }

                onNodeWithText("Code").performTextInput("abc")

                onNodeWithText("Invalid code.").assertExists()
                onNodeWithText("Connect").assertIsNotEnabled()
                captureScreenshot("short")
            }
        }

        test("valid code can connect and submits without whitespace") {
            uiTest {
                val submitted = mutableListOf<String>()
                setContentWithTestProviders { screen(isConnecting = false, onSubmit = { submitted += it }) }

                // Pasted codes can be wrapped across lines
                onNodeWithText("Code").performTextInput("${validCode.take(32)}\n ${validCode.drop(32)}")

                onNodeWithText("Invalid code.").assertDoesNotExist()
                onNodeWithText("Connect").assertIsEnabled()
                captureScreenshot("valid")

                onNodeWithText("Connect").performClick()
                submitted shouldBe listOf(validCode)
            }
        }

        test("cannot connect while connecting") {
            uiTest {
                setContentWithTestProviders { screen(isConnecting = true) }

                onNodeWithText("Code").performTextInput(validCode)

                onNodeWithText("Connect").assertIsNotEnabled()
                captureScreenshot("connecting")
            }
        }
    })

@Composable
private fun screen(
    isConnecting: Boolean,
    onSubmit: (String) -> Unit = {},
) {
    Theme {
        ConnectManuallyScreen(
            snackbarHost = {},
            onShowNodeStatus = {},
            isConnecting = isConnecting,
            onSubmit = onSubmit,
            onCancel = {},
        )
    }
}
