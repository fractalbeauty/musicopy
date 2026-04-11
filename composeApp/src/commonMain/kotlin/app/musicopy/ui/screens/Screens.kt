package app.musicopy.ui.screens

import kotlinx.serialization.Serializable

@Serializable
object Home

@Serializable
object Settings

@Serializable
object ConnectQR

@Serializable
object ConnectManually

@Serializable
data class Waiting(val nodeId: String)

@Serializable
data class PreTransfer(val nodeId: String)

@Serializable
data class Transfer(val nodeId: String)

@Serializable
data class Disconnected(val nodeId: String)
