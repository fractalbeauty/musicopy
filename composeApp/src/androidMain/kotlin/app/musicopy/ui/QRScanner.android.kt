package app.musicopy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning

@Composable
actual fun QRScanner(autoLaunch: Boolean, onResult: (String) -> Unit) {
    var resultText by remember { mutableStateOf<String?>(null) }

    val context = LocalContext.current

    val startScan = {
        val options = GmsBarcodeScannerOptions.Builder()
            .setBarcodeFormats(
                Barcode.FORMAT_QR_CODE,
            )
            .build()

        val scanner = GmsBarcodeScanning.getClient(context, options)

        scanner.startScan()
            .addOnSuccessListener { barcode ->
                // clear result text
                resultText = null

                // call callback
                val result = barcode.rawValue ?: ""
                onResult(result)
            }
            .addOnCanceledListener {
                // clear result text
                resultText = null
            }
            .addOnFailureListener { e ->
                // show error text
                resultText = "Error: $e"
            }
    }

    LaunchedEffect(Unit) {
        if (autoLaunch) {
            startScan()
        }
    }

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Button(onClick = { startScan() }) {
            Text("Scan")
        }

        resultText?.let { resultText ->
            Text(
                text = resultText,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.labelLarge
            )
        }
    }
}