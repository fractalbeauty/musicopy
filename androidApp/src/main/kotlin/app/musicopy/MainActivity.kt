package app.musicopy

import android.app.NotificationManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen

class MainActivity : ComponentActivity() {
    lateinit var observer: AppLifecycleObserver

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()

        val splashScreen = installSplashScreen()

        super.onCreate(savedInstanceState)

        // show splash screen until core is ready
        val app = application as AppApplication
        splashScreen.setKeepOnScreenCondition { !app.coreInstanceReady.value }

        // register activity lifecycle observer
        observer = AppLifecycleObserver(activityResultRegistry, contentResolver, app.appSettings)
        lifecycle.addObserver(observer)

        // cancel transfer notification
        val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        notificationManager.cancel(NOTIFICATION_ID_TRANSFER)

        val platformActivityContext = PlatformActivityContext(this, observer)

        setContent {
            val coreInstanceReady by app.coreInstanceReady.collectAsState()
            if (coreInstanceReady) {
                App(
                    platformAppContext = app.platformAppContext,
                    platformActivityContext = platformActivityContext,
                    coreInstance = app.coreInstance,
                    appSettings = app.appSettings,
                )
            }
        }
    }
}
