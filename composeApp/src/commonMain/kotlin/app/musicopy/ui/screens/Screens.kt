package app.musicopy.ui.screens

import kotlinx.serialization.Serializable

@Serializable
object Home

@Serializable
object Settings

@Serializable
object Feedback

@Serializable
object ConnectQR

@Serializable
object ConnectManually

@Serializable
data class Waiting(val endpointId: String)

@Serializable
data class PreTransfer(val endpointId: String)

@Serializable
data class Transfer(val endpointId: String)

@Serializable
data class Disconnected(val endpointId: String)
