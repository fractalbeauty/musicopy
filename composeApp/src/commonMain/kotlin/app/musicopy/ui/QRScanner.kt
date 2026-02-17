package app.musicopy.ui

import androidx.compose.runtime.Composable

@Composable
expect fun QRScanner(autoLaunch: Boolean, onResult: (String) -> Unit);
