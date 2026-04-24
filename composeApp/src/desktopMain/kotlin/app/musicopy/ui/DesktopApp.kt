package app.musicopy.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import app.musicopy.AppSettings
import app.musicopy.CoreInstance
import app.musicopy.PlatformActivityContext
import app.musicopy.PlatformAppContext

@Composable
fun DesktopApp(
    platformAppContext: PlatformAppContext,
    platformActivityContext: PlatformActivityContext,
    coreInstance: CoreInstance,
    appSettings: AppSettings,
) {
    val libraryModel by coreInstance.libraryState.collectAsState()
    val nodeModel by coreInstance.nodeState.collectAsState()
    val statsModel by coreInstance.statsState.collectAsState()

    Theme {
        Box(modifier = Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.background)) {
            DesktopHome(
                appSettings = appSettings,
                libraryModel = libraryModel,
                nodeModel = nodeModel,
                statsModel = statsModel,
                showHints = true,
                onAcceptAndTrust = { endpointId ->
                    coreInstance.instance.acceptConnectionAndTrust(
                        endpointId
                    )
                },
                onAcceptOnce = { endpointId -> coreInstance.instance.acceptConnection(endpointId) },
                onDeny = { endpointId -> coreInstance.instance.denyConnection(endpointId) },
                onAddLibraryRoot = { name, path ->
                    coreInstance.instance.addLibraryRoot(
                        name,
                        path
                    )
                },
                onRemoveLibraryRoot = { name -> coreInstance.instance.removeLibraryRoot(name) },
                onRescanLibrary = { coreInstance.instance.rescanLibrary() },
                onDeleteUnusedTranscodes = {
                    coreInstance.instance.deleteUnusedTranscodes()
                },
                onDeleteAllTranscodes = {
                    coreInstance.instance.deleteAllTranscodes()
                },
                onUntrustNode = { endpointId ->
                    coreInstance.instance.untrustNode(endpointId)
                }
            )
        }
    }
}
