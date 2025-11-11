package app.musicopy

import android.app.ForegroundServiceStartNotAllowedException
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import uniffi.musicopy.logDebug
import uniffi.musicopy.logError

class AppForegroundService : Service() {
    var started = false

    override fun onCreate() {}

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action

        val countTotal = intent?.getIntExtra("count_total", 0) ?: 0
        val countWaiting = intent?.getIntExtra("count_waiting", 0) ?: 0
        val countFailed = intent?.getIntExtra("count_failed", 0) ?: 0
        val countFinished = intent?.getIntExtra("count_finished", 0) ?: 0

        try {
            when (action) {
                "MUSICOPY_UPDATE" -> {
                    updateForeground(
                        countTotal, countWaiting, countFailed, countFinished
                    )
                }

                "MUSICOPY_STOP" -> {
                    stopForeground(
                        countTotal, countWaiting, countFailed, countFinished
                    )
                }

                else -> {
                    logError("AppForegroundService: unknown action $action")
                }
            }
        } catch (e: Exception) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                && e is ForegroundServiceStartNotAllowedException
            ) {
                logError("AppForegroundService: caught ForegroundServiceStartNotAllowedException: $e")
            }

            logError("AppForegroundService: onStartCommand error, action=$action: $e")
        }

        return START_NOT_STICKY
    }

    override fun onDestroy() {}

    override fun onBind(p0: Intent?): IBinder? = null

    fun updateForeground(
        countTotal: Int,
        countWaiting: Int,
        countFailed: Int,
        countFinished: Int,
    ) {
        // build notification
        var contentText = "$countWaiting remaining"
        if (countFailed > 0) {
            contentText += ", $countFailed failed"
        }

        val notification = NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID_FOREGROUND)
            .setSmallIcon(R.drawable.icon_mask)
            .setColor(0xff4c662b.toInt())
            .setContentTitle("Transferring $countTotal files")
            .setContentText(contentText)
            .setOngoing(true)
            .setProgress(countTotal, countFinished, false)
            .setOnlyAlertOnce(true)
            .setPriority(NotificationManager.IMPORTANCE_LOW)
            .build()

        if (!started) {
            // start foreground service using notification
            ServiceCompat.startForeground(
                this,
                100,
                notification,
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                } else {
                    0
                }
            )
            started = true

            logDebug("AppForegroundService: started foreground service")
        } else {
            // update notification
            val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
            notificationManager.notify(NOTIFICATION_ID_TRANSFER, notification)

            logDebug("AppForegroundService: updated notification")
        }
    }

    fun stopForeground(
        countTotal: Int,
        countWaiting: Int,
        countFailed: Int,
        countFinished: Int,
    ) {
        ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_DETACH)
        started = false

        // update notification
        var contentText = "Transferred $countFinished files"
        if (countFailed > 0) {
            contentText += ", $countFailed failed"
        }

        val notification = NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID_FOREGROUND)
            .setSmallIcon(R.drawable.icon_mask)
            .setColor(0xff4c662b.toInt())
            .setContentTitle("Transfer finished")
            .setContentText(contentText)
            .setOngoing(false)
            .setPriority(NotificationManager.IMPORTANCE_DEFAULT)
            .build()

        val notificationManager = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        notificationManager.notify(NOTIFICATION_ID_TRANSFER, notification)

        logDebug("AppForegroundService: stopped foreground service")
    }
}
