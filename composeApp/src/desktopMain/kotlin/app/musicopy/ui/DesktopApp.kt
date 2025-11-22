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
) {
    val libraryModel by coreInstance.libraryState.collectAsState()
    val nodeModel by coreInstance.nodeState.collectAsState()

    Theme {
        Box(modifier = Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.background)) {
            DesktopHome(
                libraryModel = libraryModel,
                nodeModel = nodeModel,
                showHints = true,
                onAcceptAndTrust = { nodeId ->
                    coreInstance.instance.acceptConnectionAndTrust(
                        nodeId
                    )
                },
                onAcceptOnce = { nodeId -> coreInstance.instance.acceptConnection(nodeId) },
                onDeny = { nodeId -> coreInstance.instance.denyConnection(nodeId) },
                onAddLibraryRoot = { name, path ->
                    coreInstance.instance.addLibraryRoot(
                        name,
                        path
                    )
                },
                onRemoveLibraryRoot = { name -> coreInstance.instance.removeLibraryRoot(name) },
                onRescanLibrary = { coreInstance.instance.rescanLibrary() },
                onSetTranscodePolicy = { policy ->
                    AppSettings.transcodePolicy = policy
                    coreInstance.instance.setTranscodePolicy(policy)
                },
                onDeleteUnusedTranscodes = {
                    coreInstance.instance.deleteUnusedTranscodes()
                },
                onDeleteAllTranscodes = {
                    coreInstance.instance.deleteAllTranscodes()
                },
                onUntrustNode = { nodeId ->
                    coreInstance.instance.untrustNode(nodeId)
                }
            )
        }
    }
}
