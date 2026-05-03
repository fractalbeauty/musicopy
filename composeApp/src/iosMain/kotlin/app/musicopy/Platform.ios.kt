package app.musicopy

import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import com.russhwolf.settings.NSUserDefaultsSettings
import com.russhwolf.settings.Settings
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import platform.Foundation.NSData
import platform.Foundation.NSError
import platform.Foundation.NSNumber
import platform.Foundation.NSNumberFormatter
import platform.Foundation.create
import platform.MessageUI.MFMailComposeResult
import platform.MessageUI.MFMailComposeViewController
import platform.MessageUI.MFMailComposeViewControllerDelegateProtocol
import platform.UIKit.UIApplication
import platform.UIKit.UIDevice
import platform.darwin.NSObject

actual val isAndroid = false

actual class PlatformAppContext {
    actual val systemDetails
        get() = buildString {
            appendLine("Platform: ${UIDevice.currentDevice.systemName} ${UIDevice.currentDevice.systemVersion}")
            appendLine("Device: ${UIDevice.currentDevice.model}")
        }

    actual val settingsFactory: Settings.Factory = NSUserDefaultsSettings.Factory()
}

actual class PlatformActivityContext

private var mailDelegate: MFMailComposeViewControllerDelegateProtocol? = null

@OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)
actual fun PlatformActivityContext.sendFeedbackEmail(
    description: String,
    logs: ByteArray,
    filename: String,
    onError: (Exception) -> Unit,
) {
    val logsLength = logs.size.toULong()
    val data = logs.usePinned { logs ->
        NSData.create(
            bytes = logs.addressOf(0),
            length = logsLength
        )
    }

    val viewController = UIApplication.sharedApplication.keyWindow?.rootViewController
        ?: throw Exception("viewController is null")

    if (MFMailComposeViewController.canSendMail()) {
        val mail = MFMailComposeViewController()
        mail.setToRecipients(listOf("support@musicopy.app"))
        mail.setSubject("Feedback")
        mail.setMessageBody(description, isHTML = false)
        mail.addAttachmentData(data, "text/plain", filename)

        mailDelegate = object : NSObject(), MFMailComposeViewControllerDelegateProtocol {
            override fun mailComposeController(
                controller: MFMailComposeViewController,
                didFinishWithResult: MFMailComposeResult,
                error: NSError?
            ) {
                when (didFinishWithResult) {
                    MFMailComposeResult.MFMailComposeResultSent -> {}

                    else -> {
                        onError(
                            Exception("mailComposeDelegate got MFMailComposeResult: $didFinishWithResult")
                        )
                    }
                }

                controller.dismissViewControllerAnimated(true, completion = null)
                mailDelegate = null
            }
        }
        mail.mailComposeDelegate = mailDelegate

        viewController.presentViewController(mail, animated = true, completion = null)
    } else {
        throw Exception("MFMailComposeViewController.canSendMail() returned false")
    }
}

actual object CoreProvider : ICoreProvider

@OptIn(ExperimentalComposeUiApi::class)
actual fun toClipEntry(string: String): ClipEntry = ClipEntry.withPlainText(string)

actual fun formatFloat(f: Float, decimals: Int): String {
    val formatter = NSNumberFormatter()
    formatter.minimumFractionDigits = 0u
    formatter.maximumFractionDigits = decimals.toULong()
    formatter.numberStyle = 1u // Decimal
    return formatter.stringFromNumber(NSNumber(f))!!
}

@Composable
actual fun rememberNotificationsPermission(): MutableState<PermissionState> {
    return stubRememberNotificationsPermission()
}

@Composable
actual fun BackHandler(enabled: Boolean, onBack: () -> Unit) {
    // not implemented on iOS
}
