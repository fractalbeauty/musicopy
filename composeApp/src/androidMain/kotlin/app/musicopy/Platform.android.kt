package app.musicopy

import android.Manifest
import android.content.ClipData
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.icu.text.DecimalFormat
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.toClipEntry
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import androidx.core.net.toUri
import com.russhwolf.settings.Settings
import com.russhwolf.settings.SharedPreferencesSettings
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.ProjectDirsOptions
import java.io.File

actual val isAndroid = true

actual class PlatformAppContext {
    actual val systemDetails
        get() = buildString {
            appendLine("Platform: Android ${Build.VERSION.SDK_INT}")
            appendLine("Device: ${Build.MANUFACTURER} ${Build.MODEL}")
        }

    actual val settingsFactory: Settings.Factory

    val application: AppApplication

    constructor(application: AppApplication) {
        this.application = application
        this.settingsFactory = SharedPreferencesSettings.Factory(this.application)
    }
}

actual class PlatformActivityContext {
    val mainActivity: MainActivity

    constructor(mainActivity: MainActivity) {
        this.mainActivity = mainActivity
    }
}

actual object CoreProvider : ICoreProvider {
    override fun getOptions(
        platformAppContext: PlatformAppContext,
        appSettings: AppSettings,
    ): CoreOptions {
        val options = super.getOptions(platformAppContext, appSettings)
        options.projectDirs = ProjectDirsOptions(
            dataDir = platformAppContext.application.filesDir.path,
            cacheDir = platformAppContext.application.cacheDir.path
        )
        return options
    }
}

actual fun PlatformActivityContext.sendFeedbackEmail(
    description: String,
    logs: ByteArray,
    filename: String,
) {
    // Write log bytes to a file in the cache dir and get a URI to attach to the intent
    val file = File(mainActivity.cacheDir, filename)
    file.writeBytes(logs)
    val uri = FileProvider.getUriForFile(
        mainActivity, "app.musicopy.fileprovider", file
    )

    // Query for activities supporting email intents. This ensures we only show dedicated email
    // clients and not other apps that can also send things.
    val selectorIntent = Intent(Intent.ACTION_SENDTO, "mailto:".toUri())
    val resolvedActivities = mainActivity.packageManager.queryIntentActivities(selectorIntent, 0)

    // Map each resolved activities to an intent for that activity.
    //
    // Without this (using a single intent without EXTRA_INITIAL_INTENTS, and setting a selector
    // instead of querying activities), Thunderbird was not attaching the file. Gmail and Outlook
    // worked, but something about the selector and chooser intents caused Thunderbird to not get
    // the file.
    val intents = resolvedActivities.map { resolveInfo ->
        Intent(Intent.ACTION_SEND).apply {
            type = "message/rfc822"
            putExtra(Intent.EXTRA_EMAIL, arrayOf("support@musicopy.app"))
            putExtra(Intent.EXTRA_SUBJECT, "Feedback")
            putExtra(Intent.EXTRA_TEXT, description)
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            component = ComponentName(
                resolveInfo.activityInfo.packageName,
                resolveInfo.activityInfo.name
            )
        }
    }

    // Create a chooser intent from the resolved activities.
    if (intents.isNotEmpty()) {
        val chooser = Intent.createChooser(intents.first(), "Send email")
        if (intents.size > 1) {
            chooser.putExtra(Intent.EXTRA_INITIAL_INTENTS, intents.drop(1).toTypedArray())
        }
        mainActivity.startActivity(chooser)
    } else {
        throw Exception("No activity available to send email")
    }
}

actual fun toClipEntry(string: String): ClipEntry =
    ClipData.newPlainText("label", string).toClipEntry()

actual fun formatFloat(f: Float, decimals: Int): String {
    val df = DecimalFormat()
    df.maximumFractionDigits = decimals
    return df.format(f)
}

@Composable
actual fun rememberNotificationsPermission(): MutableState<PermissionState> {
    val context = LocalContext.current
    val isGranted = remember {
        val initialValue = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            when (
                ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS)
            ) {
                PackageManager.PERMISSION_GRANTED -> true
                PackageManager.PERMISSION_DENIED -> false
                else -> false
            }
        } else {
            true
        }

        mutableStateOf(initialValue)
    }

    val launcher =
        rememberLauncherForActivityResult(
            ActivityResultContracts.RequestPermission()
        ) { value ->
            isGranted.value = value
        }

    val requestPermission = {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
    }

    val permissionState =
        remember {
            mutableStateOf(
                PermissionState(
                    isGranted = isGranted.value,
                    requestPermission = requestPermission
                )
            )
        }
    LaunchedEffect(isGranted.value) {
        permissionState.value = PermissionState(
            isGranted = isGranted.value,
            requestPermission = requestPermission
        )
    }

    return permissionState
}

@Composable
actual fun BackHandler(enabled: Boolean, onBack: () -> Unit) {
    androidx.activity.compose.BackHandler(enabled = enabled, onBack = onBack)
}
