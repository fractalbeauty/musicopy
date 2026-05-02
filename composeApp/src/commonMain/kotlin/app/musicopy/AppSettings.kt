@file:OptIn(ExperimentalSettingsApi::class)

package app.musicopy

import com.russhwolf.settings.ExperimentalSettingsApi
import com.russhwolf.settings.MapSettings
import com.russhwolf.settings.ObservableSettings
import com.russhwolf.settings.coroutines.getBooleanFlow
import com.russhwolf.settings.coroutines.getLongOrNullFlow
import com.russhwolf.settings.coroutines.getStringFlow
import com.russhwolf.settings.coroutines.getStringOrNullFlow
import com.russhwolf.settings.observable.makeObservable
import kotlinx.coroutines.flow.Flow
import uniffi.musicopy.logInfo

const val DOWNLOAD_DIRECTORY_KEY = "downloadDirectory"
const val DOWNLOAD_DIRECTORY_NAME_KEY = "downloadDirectoryName"
const val DETAILED_ERRORS_KEY = "detailedErrors"
const val LICENSE_KEY_KEY = "licenseKey"
const val LICENSE_ACTIVATED_AT_KEY = "licenseActivatedAt"
const val TRANSCODE_FORMAT_KEY = "transcodeFormat"

const val defaultTranscodeFormat = "opus128"

class AppSettings private constructor(private val settings: ObservableSettings) {
    constructor(platformAppContext: PlatformAppContext) : this(
        settings = platformAppContext.settingsFactory.create().makeObservable()
    )

    companion object {
        fun createMock(): AppSettings {
            return AppSettings(settings = MapSettings())
        }
    }

    fun clearSettings() {
        // License key is not cleared
        settings.remove(DOWNLOAD_DIRECTORY_KEY)
        settings.remove(DOWNLOAD_DIRECTORY_NAME_KEY)
        settings.remove(DETAILED_ERRORS_KEY)
        settings.remove(TRANSCODE_FORMAT_KEY)

        logInfo("AppSettings: cleared settings")
    }

    var downloadDirectory: String?
        get() = settings.getStringOrNull(DOWNLOAD_DIRECTORY_KEY)
        set(value) {
            value?.let {
                settings.putString(DOWNLOAD_DIRECTORY_KEY, value)
            } ?: run {
                settings.remove(DOWNLOAD_DIRECTORY_KEY)
            }
        }

    val downloadDirectoryFlow: Flow<String?>
        get() = settings.getStringOrNullFlow(DOWNLOAD_DIRECTORY_KEY)

    var downloadDirectoryName: String?
        get() = settings.getStringOrNull(DOWNLOAD_DIRECTORY_NAME_KEY)
        set(value) {
            value?.let {
                settings.putString(DOWNLOAD_DIRECTORY_NAME_KEY, value)
            } ?: run {
                settings.remove(DOWNLOAD_DIRECTORY_NAME_KEY)
            }
        }

    val downloadDirectoryNameFlow: Flow<String?>
        get() = settings.getStringOrNullFlow(DOWNLOAD_DIRECTORY_NAME_KEY)

    var detailedErrors: Boolean
        get() = settings.getBoolean(DETAILED_ERRORS_KEY, false)
        set(value) {
            settings.putBoolean(DETAILED_ERRORS_KEY, value)
        }

    val detailedErrorsFlow: Flow<Boolean>
        get() = settings.getBooleanFlow(DETAILED_ERRORS_KEY, false)

    var licenseKey: String?
        get() = settings.getStringOrNull(LICENSE_KEY_KEY)
        set(value) {
            value?.let {
                settings.putString(LICENSE_KEY_KEY, value)
            } ?: run {
                settings.remove(LICENSE_KEY_KEY)
            }
        }

    val licenseKeyFlow: Flow<String?> = settings.getStringOrNullFlow(LICENSE_KEY_KEY)

    /**
     * License activation timestamp, in seconds.
     */
    var licenseActivatedAt: Long?
        get() = settings.getLongOrNull(LICENSE_ACTIVATED_AT_KEY)
        set(value) {
            value?.let {
                settings.putLong(LICENSE_ACTIVATED_AT_KEY, value)
            } ?: run {
                settings.remove(LICENSE_ACTIVATED_AT_KEY)
            }
        }

    /**
     * License activation timestamp, in seconds.
     */
    val licenseActivatedAtFlow: Flow<Long?> = settings.getLongOrNullFlow(LICENSE_ACTIVATED_AT_KEY)

    var transcodeFormat: String
        get() = settings.getString(TRANSCODE_FORMAT_KEY, defaultTranscodeFormat)
        set(value) {
            settings.putString(TRANSCODE_FORMAT_KEY, value)
        }

    val transcodeFormatFlow: Flow<String> =
        settings.getStringFlow(TRANSCODE_FORMAT_KEY, defaultTranscodeFormat)
}
