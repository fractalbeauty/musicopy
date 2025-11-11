package app.musicopy

import android.Manifest
import android.content.ClipData
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
import uniffi.musicopy.CoreOptions
import uniffi.musicopy.ProjectDirsOptions

actual val isAndroid = true

actual class PlatformAppContext private actual constructor() {
    actual val name: String = "Android ${Build.VERSION.SDK_INT}"

    lateinit var application: AppApplication

    constructor(application: AppApplication) : this() {
        this.application = application
    }
}

actual class PlatformActivityContext private actual constructor() {
    lateinit var mainActivity: MainActivity
        private set

    constructor(mainActivity: MainActivity) : this() {
        this.mainActivity = mainActivity
    }
}

actual object CoreProvider : ICoreProvider {
    override fun getOptions(platformAppContext: PlatformAppContext): CoreOptions {
        val options = super.getOptions(platformAppContext)
        options.projectDirs = ProjectDirsOptions(
            dataDir = platformAppContext.application.filesDir.path,
            cacheDir = platformAppContext.application.cacheDir.path
        )
        return options
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
