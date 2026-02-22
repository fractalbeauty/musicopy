package app.musicopy

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import uniffi.musicopy.Core
import uniffi.musicopy.EventHandler
import uniffi.musicopy.LibraryModel
import uniffi.musicopy.NodeModel

class CoreInstance private constructor() : EventHandler {
    companion object {
        suspend fun start(platformAppContext: PlatformAppContext, appSettings: AppSettings): CoreInstance {
            val instance = CoreInstance()
            instance._instance = Core.start(
                eventHandler = instance,
                options = CoreProvider.getOptions(platformAppContext, appSettings)
            )
            instance._libraryState = MutableStateFlow(instance._instance.getLibraryModel())
            instance._nodeState = MutableStateFlow(instance._instance.getNodeModel())
            return instance
        }
    }

    private lateinit var _instance: Core
    val instance: Core
        get() = _instance

    private lateinit var _libraryState: MutableStateFlow<LibraryModel>

    val libraryState: StateFlow<LibraryModel>
        get() = _libraryState

    private lateinit var _nodeState: MutableStateFlow<NodeModel>
    val nodeState: StateFlow<NodeModel>
        get() = _nodeState

    override fun onLibraryModelSnapshot(model: LibraryModel) {
        // TODO: this is a hack because Core.start calls the callback before CoreInstance finishes initializing
        if (::_libraryState.isInitialized) {
            _libraryState.value = model
        }
    }

    override fun onNodeModelSnapshot(model: NodeModel) {
        if (::_nodeState.isInitialized) {
            _nodeState.value = model
        }
    }
}