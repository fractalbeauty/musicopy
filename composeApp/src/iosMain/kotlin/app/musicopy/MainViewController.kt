package app.musicopy

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.MutableState
import androidx.compose.ui.window.ComposeUIViewController
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import platform.UIKit.UIViewController

fun MainViewController(): UIViewController {
    val platformAppContext = PlatformAppContext()
    val platformActivityContext = PlatformActivityContext()
    val appSettings = AppSettings()

    val coreInstanceState: MutableState<CoreInstance?> = mutableStateOf(null)
    GlobalScope.launch {
        coreInstanceState.value = CoreInstance.start(platformAppContext, appSettings)
    }

    return ComposeUIViewController {
        var coreInstance by coreInstanceState

        coreInstance?.let { coreInstance ->
            App(
                platformAppContext = platformAppContext,
                platformActivityContext = platformActivityContext,
                coreInstance = coreInstance,
                appSettings = appSettings
            )
        }
    }
}
