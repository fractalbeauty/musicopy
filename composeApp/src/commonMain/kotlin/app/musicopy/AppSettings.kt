@file:OptIn(ExperimentalSettingsApi::class)

package app.musicopy

import com.russhwolf.settings.ExperimentalSettingsApi
import com.russhwolf.settings.MapSettings
import com.russhwolf.settings.ObservableSettings
import com.russhwolf.settings.coroutines.getBooleanFlow
import com.russhwolf.settings.coroutines.getBooleanStateFlow
import com.russhwolf.settings.coroutines.getStringOrNullFlow
import com.russhwolf.settings.observable.makeObservable
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import uniffi.musicopy.TranscodePolicy

const val DOWNLOAD_DIRECTORY_KEY = "downloadDirectory"
const val TRANSCODE_POLICY_KEY = "transcodePolicy"
const val DETAILED_ERRORS_KEY = "detailedErrors"

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
        settings.clear()
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

    var transcodePolicy: TranscodePolicy
        get() = deserializeTranscodePolicy(settings.getStringOrNull(TRANSCODE_POLICY_KEY))
        set(value) {
            settings.putString(TRANSCODE_POLICY_KEY, serializeTranscodePolicy(value))
        }

    val transcodePolicyFlow: Flow<TranscodePolicy>
        get() = settings.getStringOrNullFlow(TRANSCODE_POLICY_KEY)
            .map { deserializeTranscodePolicy(it) }

    var detailedErrors: Boolean
        get() = settings.getBoolean(DETAILED_ERRORS_KEY, false)
        set(value) {
            settings.putBoolean(DETAILED_ERRORS_KEY, value)
        }

    val detailedErrorsFlow: Flow<Boolean>
        get() = settings.getBooleanFlow(DETAILED_ERRORS_KEY, false)
}

internal fun deserializeTranscodePolicy(s: String?) = when (s) {
    "IF_REQUESTED" -> TranscodePolicy.IF_REQUESTED
    "ALWAYS" -> TranscodePolicy.ALWAYS
    else -> TranscodePolicy.IF_REQUESTED
}

internal fun serializeTranscodePolicy(p: TranscodePolicy) = when (p) {
    TranscodePolicy.IF_REQUESTED -> "IF_REQUESTED"
    TranscodePolicy.ALWAYS -> "ALWAYS"
}
