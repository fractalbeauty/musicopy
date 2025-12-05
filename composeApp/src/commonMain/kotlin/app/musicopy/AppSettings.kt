@file:OptIn(ExperimentalSettingsApi::class)

package app.musicopy

import com.russhwolf.settings.ExperimentalSettingsApi
import com.russhwolf.settings.MapSettings
import com.russhwolf.settings.ObservableSettings
import com.russhwolf.settings.coroutines.getStringOrNullFlow
import com.russhwolf.settings.observable.makeObservable
import kotlinx.coroutines.flow.Flow
import com.russhwolf.settings.Settings
import kotlinx.coroutines.flow.map
import uniffi.musicopy.TranscodePolicy

const val DOWNLOAD_DIRECTORY_KEY = "downloadDirectory"
const val TRANSCODE_POLICY_KEY = "transcodePolicy"

object AppSettings {
    private var settings: ObservableSettings = Settings().makeObservable()

    fun installMockSettings() {
        settings = MapSettings().makeObservable()
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
