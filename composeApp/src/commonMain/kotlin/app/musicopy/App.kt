package app.musicopy

import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.core.tween
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.toRoute
import app.musicopy.ui.NodeStatusSheet
import app.musicopy.ui.Theme
import app.musicopy.ui.rememberNodeStatusSheetState
import app.musicopy.ui.screens.ConnectManually
import app.musicopy.ui.screens.ConnectManuallyScreen
import app.musicopy.ui.screens.ConnectQR
import app.musicopy.ui.screens.ConnectQRScreen
import app.musicopy.ui.screens.Disconnected
import app.musicopy.ui.screens.DisconnectedScreen
import app.musicopy.ui.screens.Home
import app.musicopy.ui.screens.HomeScreen
import app.musicopy.ui.screens.PreTransfer
import app.musicopy.ui.screens.PreTransferScreen
import app.musicopy.ui.screens.Transfer
import app.musicopy.ui.screens.TransferScreen
import app.musicopy.ui.screens.Waiting
import app.musicopy.ui.screens.WaitingScreen
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.musicopy.ClientStateModel
import uniffi.musicopy.CoreException

@Composable
fun App(
    platformAppContext: PlatformAppContext,
    platformActivityContext: PlatformActivityContext,
    coreInstance: CoreInstance,
    navController: NavHostController = rememberNavController(),
) {
    val scope = rememberCoroutineScope()

    // collect core state flows as compose state
    val libraryModel by coreInstance.libraryState.collectAsState()
    val nodeModel by coreInstance.nodeState.collectAsState()

    val directoryPicker = rememberDirectoryPicker(platformActivityContext)

    // set download directory when changed
    val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(initial = null)
    LaunchedEffect(downloadDirectory) {
        downloadDirectory?.let {
            coreInstance.instance.setDownloadDirectory(it)
        }
    }

    val snackbarHostState = remember { SnackbarHostState() }
    val snackbarHost = @Composable { SnackbarHost(snackbarHostState) }

    var connectingTo by remember { mutableStateOf<String?>(null) }
    val isConnecting = connectingTo !== null

    val showErrorSnackbar = fun(message: String, e: CoreException) {
        // TODO: app setting
        val detailed = true

        scope.launch {
            if (detailed) {
                snackbarHostState.showSnackbar(
                    message = message + "\n\nError: ${e.message()}",
                    duration = SnackbarDuration.Indefinite,
                    withDismissAction = true
                )
            } else {
                snackbarHostState.showSnackbar(
                    message = message
                )
            }
        }
    }

    val onConnect = fun(nodeId: String, popToHome: Boolean) {
        // only connect to one node at a time
        if (connectingTo != null) {
            println("onConnect failed, already connecting")
            return
        }
        connectingTo = nodeId

        scope.launch {
            try {
                coreInstance.instance.connect(nodeId = nodeId)
                delay(100) // TODO
                val client = nodeModel.clients.values.find { it.nodeId == nodeId }
                if (client?.state is ClientStateModel.Accepted) {
                    navController.navigate(PreTransfer(nodeId = nodeId)) {
                        if (popToHome) {
                            popUpTo<Home> {
                                inclusive = false
                            }
                        }
                    }
                } else {
                    navController.navigate(Waiting(nodeId = nodeId)) {
                        if (popToHome) {
                            popUpTo<Home> {
                                inclusive = false
                            }
                        }
                    }
                }
            } catch (e: CoreException) {
                showErrorSnackbar("Failed to connect", e)
                println("error during connect: $e")
            } finally {
                connectingTo = null
            }
        }
        Unit
    }

    val leaveClientScreen = { nodeId: String ->
        coreInstance.instance.closeClient(nodeId)

        navController.popBackStack(Home, inclusive = false)
    }

    val nodeStatusSheetState = rememberNodeStatusSheetState()
    NodeStatusSheet(nodeStatusSheetState, nodeModel)
    val onShowNodeStatus = { nodeStatusSheetState.peek() }

    Theme {
        NavHost(
            navController = navController,
            startDestination = Home,
            // TODO
            // modifier = Modifier.verticalScroll(rememberScrollState()),
            enterTransition = {
                slideIntoContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Left,
                    animationSpec = tween(700)
                )
            },
            exitTransition = {
                slideOutOfContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Left,
                    animationSpec = tween(700)
                )
            },
            popEnterTransition = {
                slideIntoContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Right,
                    animationSpec = tween(700)
                )
            },
            popExitTransition = {
                slideOutOfContainer(
                    towards = AnimatedContentTransitionScope.SlideDirection.Right,
                    animationSpec = tween(700)
                )
            }
        ) {
            composable<Home> {
                HomeScreen(
                    snackbarHost = snackbarHost,
                    onShowNodeStatus = onShowNodeStatus,

                    recentServers = nodeModel.recentServers,
                    connectingTo = connectingTo,
                    onPickDownloadDirectory = {
                        scope.launch {
                            directoryPicker.pickDownloadDirectory()
                        }
                    },
                    onConnectQRButtonClicked = {
                        navController.navigate(ConnectQR)
                    },
                    onConnectManuallyButtonClicked = {
                        navController.navigate(ConnectManually)
                    },
                    onConnectRecent = { nodeId -> onConnect(nodeId, false) },
                )
            }
            composable<ConnectQR> {
                ConnectQRScreen(
                    snackbarHost = snackbarHost,
                    onShowNodeStatus = onShowNodeStatus,

                    isConnecting = isConnecting,
                    onSubmit = { nodeId -> onConnect(nodeId, false) },
                    onCancel = {
                        navController.popBackStack(Home, inclusive = false)
                    },
                )

            }
            composable<ConnectManually> {
                ConnectManuallyScreen(
                    snackbarHost = snackbarHost,
                    onShowNodeStatus = onShowNodeStatus,

                    isConnecting = isConnecting,
                    onSubmit = { nodeId -> onConnect(nodeId, false) },
                    onCancel = {
                        navController.popBackStack(Home, inclusive = false)
                    }
                )

            }
            composable<Waiting> { backStackEntry ->
                val waiting: Waiting = backStackEntry.toRoute()
                val nodeId = waiting.nodeId
                val clientModel = nodeModel.clients.values.find { x -> x.nodeId == nodeId }

                // if still on this route
                if (navController.currentDestination?.hasRoute<Waiting>() == true) {
                    // if accepted
                    if (clientModel?.state is ClientStateModel.Accepted) {
                        navController.navigate(PreTransfer(nodeId = nodeId)) {
                            // pop Waiting screen from back stack
                            popUpTo<Waiting>() {
                                inclusive = true
                            }
                        }
                    }

                    // if closed
                    if (clientModel?.state is ClientStateModel.Closed) {
                        navController.navigate(Disconnected(nodeId = nodeId)) {
                            // pop Waiting screen from back stack
                            popUpTo<Waiting>() {
                                inclusive = true
                            }
                        }
                    }
                }

                clientModel?.let { clientModel ->
                    WaitingScreen(
                        snackbarHost = snackbarHost,
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        onCancel = {
                            leaveClientScreen(nodeId)
                        }
                    )

                }
            }
            composable<PreTransfer> { backStackEntry ->
                val preTransfer: PreTransfer = backStackEntry.toRoute()
                val nodeId = preTransfer.nodeId
                val clientModel = nodeModel.clients.values.find { x -> x.nodeId == nodeId }

                // if still on this route
                if (navController.currentDestination?.hasRoute<PreTransfer>() == true) {
                    // if closed
                    if (clientModel?.state is ClientStateModel.Closed) {
                        // navigate to Disconnected screen
                        navController.navigate(Disconnected(nodeId = nodeId)) {
                            // clear back stack
                            popUpTo<Home> {
                                inclusive = false
                            }
                        }
                    }
                }

                // TODO: handle null better, redirect to error screen?
                if (clientModel != null) {
                    val downloadDirectory by AppSettings.downloadDirectoryFlow.collectAsState(
                        null
                    )

                    PreTransferScreen(
                        snackbarHost = snackbarHost,
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        hasDownloadDirectory = downloadDirectory != null,
                        onPickDownloadDirectory = {
                            scope.launch {
                                directoryPicker.pickDownloadDirectory()
                            }
                        },
                        onDownloadAll = {
                            downloadDirectory?.let { downloadDirectory ->
                                coreInstance.instance.downloadAll(nodeId)
                                navController.navigate(Transfer(nodeId = nodeId))
                            } ?: run {
                                // TODO toast?
                                println("download directory is null")
                            }
                        },
                        onDownloadPartial = { items ->
                            downloadDirectory?.let { downloadDirectory ->
                                coreInstance.instance.downloadPartial(
                                    nodeId,
                                    items,
                                )
                                navController.navigate(Transfer(nodeId = nodeId))
                            } ?: run {
                                // TODO toast?
                                println("download directory is null")
                            }
                        },
                        onCancel = {
                            leaveClientScreen(nodeId)
                        }
                    )
                }
            }
            composable<Transfer> { backStackEntry ->
                val transfer: Transfer = backStackEntry.toRoute()
                val nodeId = transfer.nodeId
                val clientModel = nodeModel.clients.values.find { x -> x.nodeId == nodeId }

                // if still on this route
                if (navController.currentDestination?.hasRoute<Transfer>() == true) {
                    // if closed
                    if (clientModel?.state is ClientStateModel.Closed) {
                        // navigate to Disconnected screen
                        navController.navigate(Disconnected(nodeId = nodeId)) {
                            // clear back stack
                            popUpTo<Home> {
                                inclusive = false
                            }
                        }
                    }
                }

                // TODO: handle null better, redirect to error screen?
                if (clientModel != null) {
                    TransferScreen(
                        snackbarHost = snackbarHost,
                        onShowNodeStatus = onShowNodeStatus,

                        clientModel = clientModel,
                        onCancel = {
                            // pop back to pretransfer
                            navController.popBackStack(PreTransfer(nodeId), inclusive = false)
                        },
                        onTransferMore = {
                            // pop back to pretransfer
                            navController.popBackStack(PreTransfer(nodeId), inclusive = false)
                        },
                        onDone = {
                            leaveClientScreen(nodeId)
                        }
                    )
                }

            }
            composable<Disconnected> { backStackEntry ->
                val route: Disconnected = backStackEntry.toRoute()
                val nodeId = route.nodeId
                
                val clientModel = nodeModel.clients.values.find { x -> x.nodeId == nodeId }
                val name = clientModel?.name ?: "Unknown"

                DisconnectedScreen(
                    snackbarHost = snackbarHost,
                    onShowNodeStatus = onShowNodeStatus,

                    nodeId = nodeId,
                    name = name,
                    isConnecting = isConnecting,
                    onReconnect = { onConnect(nodeId, true) },
                    onCancel = {
                        // pop back to home
                        navController.popBackStack(Home, inclusive = false)
                    }
                )
            }
        }
    }
}
