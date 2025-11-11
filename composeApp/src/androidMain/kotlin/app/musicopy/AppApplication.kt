package app.musicopy

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.os.Build
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import uniffi.musicopy.ClientStateModel
import uniffi.musicopy.TransferJobProgressModel
import uniffi.musicopy.logDebug

const val NOTIFICATION_CHANNEL_ID_FOREGROUND = "foreground"
const val NOTIFICATION_ID_TRANSFER = 100

class AppApplication : Application() {
    var platformAppContext: PlatformAppContext = PlatformAppContext(this)

    lateinit var coreInstance: CoreInstance
        private set
    val coreInstanceReady = MutableStateFlow(false)

    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)

        // initialize ndk_context crate
        RustNdkContext.init(this)
    }

    override fun onCreate() {
        super.onCreate()

        // launch coroutine to initialize core instance asynchronously
        @OptIn(DelicateCoroutinesApi::class)
        GlobalScope.launch {
            coreInstance = CoreInstance.start(platformAppContext)
            coreInstanceReady.value = true

            onCoreInstanceReady()
        }

        // create notification channels
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val foregroundNotificationChannel =
                NotificationChannel(
                    NOTIFICATION_CHANNEL_ID_FOREGROUND,
                    "Transfers",
                    NotificationManager.IMPORTANCE_DEFAULT
                )

            val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
            notificationManager.createNotificationChannel(foregroundNotificationChannel)
        }
    }

    fun onCoreInstanceReady() {
        // launch coroutine to send transfer progress to foreground service
        @OptIn(DelicateCoroutinesApi::class)
        GlobalScope.launch {
            var started = false

            coreInstance.nodeState
                .collect { nodeState ->
                    // count job statuses
                    var countTotal = 0
                    var countWaiting = 0
                    var countFinished = 0
                    var countFailed = 0
                    nodeState.clients.values.forEach { clientModel ->
                        // if not accepted, count as failed
                        if (clientModel.state is ClientStateModel.Accepted) {
                            clientModel.transferJobs.forEach { transferJobModel ->
                                countTotal += 1

                                when (transferJobModel.progress) {
                                    is TransferJobProgressModel.Requested -> countWaiting += 1
                                    is TransferJobProgressModel.Transcoding -> countWaiting += 1
                                    is TransferJobProgressModel.Ready -> countWaiting += 1
                                    is TransferJobProgressModel.InProgress -> countWaiting += 1

                                    is TransferJobProgressModel.Finished -> countFinished += 1

                                    is TransferJobProgressModel.Failed -> countFailed += 1
                                }
                            }
                        } else {
                            clientModel.transferJobs.forEach { transferJobModel ->
                                countTotal += 1

                                when (transferJobModel.progress) {
                                    is TransferJobProgressModel.Requested -> countFailed += 1
                                    is TransferJobProgressModel.Transcoding -> countFailed += 1
                                    is TransferJobProgressModel.Ready -> countFailed += 1
                                    is TransferJobProgressModel.InProgress -> countFailed += 1

                                    is TransferJobProgressModel.Finished -> countFinished += 1

                                    is TransferJobProgressModel.Failed -> countFailed += 1
                                }
                            }
                        }
                    }

                    val serviceIntent =
                        Intent(this@AppApplication, AppForegroundService::class.java)

                    // send progress data in extras
                    serviceIntent.putExtra("count_total", countTotal)
                    serviceIntent.putExtra("count_waiting", countWaiting)
                    serviceIntent.putExtra("count_finished", countFinished)
                    serviceIntent.putExtra("count_failed", countFailed)

                    if (countWaiting == 0) {
                        // stop the service if everything is settled and it's currently started
                        if (started) {
                            serviceIntent.action = "MUSICOPY_STOP"

                            applicationContext.startService(serviceIntent)
                            started = false
                        }
                    } else {
                        serviceIntent.action = "MUSICOPY_UPDATE"

                        // start the service or update the notification
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                            applicationContext.startForegroundService(serviceIntent)
                        } else {
                            applicationContext.startService(serviceIntent)
                        }

                        started = true

                        // wait before processing again
                        delay(1000)
                    }
                }
        }
    }
}
