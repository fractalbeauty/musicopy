@file:OptIn(ExperimentalForeignApi::class)

package app.musicopy.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.viewinterop.UIKitView
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.readValue
import platform.AVFoundation.AVAuthorizationStatusAuthorized
import platform.AVFoundation.AVAuthorizationStatusDenied
import platform.AVFoundation.AVAuthorizationStatusNotDetermined
import platform.AVFoundation.AVAuthorizationStatusRestricted
import platform.AVFoundation.AVCaptureConnection
import platform.AVFoundation.AVCaptureDevice
import platform.AVFoundation.AVCaptureDeviceDiscoverySession.Companion.discoverySessionWithDeviceTypes
import platform.AVFoundation.AVCaptureDeviceInput.Companion.deviceInputWithDevice
import platform.AVFoundation.AVCaptureDevicePositionBack
import platform.AVFoundation.AVCaptureDeviceTypeBuiltInDualCamera
import platform.AVFoundation.AVCaptureDeviceTypeBuiltInDualWideCamera
import platform.AVFoundation.AVCaptureDeviceTypeBuiltInTripleCamera
import platform.AVFoundation.AVCaptureDeviceTypeBuiltInWideAngleCamera
import platform.AVFoundation.AVCaptureMetadataOutput
import platform.AVFoundation.AVCaptureMetadataOutputObjectsDelegateProtocol
import platform.AVFoundation.AVCaptureOutput
import platform.AVFoundation.AVCapturePhotoOutput
import platform.AVFoundation.AVCaptureSession
import platform.AVFoundation.AVCaptureSessionPresetPhoto
import platform.AVFoundation.AVCaptureVideoPreviewLayer
import platform.AVFoundation.AVLayerVideoGravityResizeAspectFill
import platform.AVFoundation.AVMediaTypeVideo
import platform.AVFoundation.AVMetadataMachineReadableCodeObject
import platform.AVFoundation.AVMetadataObjectTypeQRCode
import platform.AVFoundation.authorizationStatusForMediaType
import platform.AVFoundation.requestAccessForMediaType
import platform.AudioToolbox.AudioServicesPlaySystemSound
import platform.AudioToolbox.kSystemSoundID_Vibrate
import platform.CoreGraphics.CGRectZero
import platform.QuartzCore.CATransaction
import platform.QuartzCore.kCATransactionDisableActions
import platform.UIKit.UIView
import platform.darwin.NSObject
import platform.darwin.dispatch_get_main_queue

private sealed interface CameraAccess {
    object Unknown : CameraAccess
    object Denied : CameraAccess
    object Authorized : CameraAccess
}

@Composable
actual fun QRScanner(onResult: (String) -> Unit) {
    var cameraAccess: CameraAccess by remember { mutableStateOf(CameraAccess.Unknown) }
    LaunchedEffect(Unit) {
        when (AVCaptureDevice.authorizationStatusForMediaType(AVMediaTypeVideo)) {
            AVAuthorizationStatusAuthorized -> {
                cameraAccess = CameraAccess.Authorized
            }

            AVAuthorizationStatusDenied, AVAuthorizationStatusRestricted -> {
                cameraAccess = CameraAccess.Denied
            }

            AVAuthorizationStatusNotDetermined -> {
                AVCaptureDevice.requestAccessForMediaType(
                    mediaType = AVMediaTypeVideo
                ) { success ->
                    cameraAccess = if (success) CameraAccess.Authorized else CameraAccess.Denied
                }
            }
        }
    }

    Box(
        modifier = Modifier.fillMaxSize().aspectRatio(1f).background(Color.Black),
        contentAlignment = Alignment.Center
    ) {
        when (cameraAccess) {
            CameraAccess.Unknown -> {
                // waiting for permission status
            }

            CameraAccess.Denied -> {
                // TODO: button or hint to open settings?
                Text("Camera access denied", color = Color.White)
            }

            CameraAccess.Authorized -> {
                CameraAuthorized(onResult = onResult)
            }
        }
    }
}

@Composable
internal fun CameraAuthorized(onResult: (String) -> Unit) {
    val camera: AVCaptureDevice? = remember {
        discoverySessionWithDeviceTypes(
            deviceTypes = listOf(
                AVCaptureDeviceTypeBuiltInTripleCamera,
                AVCaptureDeviceTypeBuiltInDualWideCamera,
                AVCaptureDeviceTypeBuiltInDualCamera,
                AVCaptureDeviceTypeBuiltInWideAngleCamera,
            ),
            mediaType = AVMediaTypeVideo,
            position = AVCaptureDevicePositionBack,
        ).devices.firstOrNull() as? AVCaptureDevice
    }
    if (camera != null) {
        CameraFound(onResult, camera)
    } else {
        // TODO: help?
        Text("Camera device not found", color = Color.White)
    }
}

@Composable
internal fun CameraFound(
    onResult: (String) -> Unit,
    camera: AVCaptureDevice
) {
    val metadataObjectsDelegate = remember {
        object : NSObject(), AVCaptureMetadataOutputObjectsDelegateProtocol {
            lateinit var captureSession: AVCaptureSession

            override fun captureOutput(
                output: AVCaptureOutput,
                didOutputMetadataObjects: List<*>,
                fromConnection: AVCaptureConnection
            ) {
                didOutputMetadataObjects.forEach { metadataObject ->
                    if (metadataObject is AVMetadataMachineReadableCodeObject) {
                        val code = metadataObject.stringValue ?: ""
                        AudioServicesPlaySystemSound(kSystemSoundID_Vibrate)
                        onResult(code)
                        captureSession.stopRunning()
                    }
                }
            }
        }
    }

    val captureSession = remember {
        val captureSession = AVCaptureSession()
        captureSession.sessionPreset = AVCaptureSessionPresetPhoto

        val captureDeviceInput = deviceInputWithDevice(device = camera, error = null)!!
        captureSession.addInput(captureDeviceInput)

        val capturePhotoOutput = AVCapturePhotoOutput()
        captureSession.addOutput(capturePhotoOutput)

        val metadataOutput = AVCaptureMetadataOutput()
        if (captureSession.canAddOutput(metadataOutput)) {
            captureSession.addOutput(metadataOutput)

            metadataOutput.setMetadataObjectsDelegate(
                metadataObjectsDelegate,
                dispatch_get_main_queue()
            )

            metadataOutput.metadataObjectTypes = listOf(AVMetadataObjectTypeQRCode)
        }

        metadataObjectsDelegate.captureSession = captureSession

        captureSession
    }

    val cameraPreviewLayer = remember { AVCaptureVideoPreviewLayer(session = captureSession) }

    UIKitView(
        modifier = Modifier.fillMaxSize(),
        factory = {
            val cameraContainer = object : UIView(frame = CGRectZero.readValue()) {
                override fun layoutSubviews() {
                    CATransaction.begin()
                    CATransaction.setValue(true, kCATransactionDisableActions)
                    layer.setFrame(frame)
                    cameraPreviewLayer.setFrame(frame)
                    CATransaction.commit()
                }
            }
            cameraContainer.layer.addSublayer(cameraPreviewLayer)
            cameraPreviewLayer.videoGravity = AVLayerVideoGravityResizeAspectFill
            captureSession.startRunning()
            cameraContainer
        },
    )
}
