package com.shashlik.kmp

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitInteropInteractionMode
import androidx.compose.ui.viewinterop.UIKitInteropProperties
import androidx.compose.ui.viewinterop.UIKitView

@OptIn(kotlin.experimental.ExperimentalNativeApi::class)
actual val isDebugBuild: Boolean get() = Platform.isDebugBinary
@Composable
actual fun ShashlikMap() {
    LaunchedEffect(Unit) {
        IOSLocationProvider(
            onLocationUpdated = { lat, lon, bearing ->
                println("Success! GPS Coordinates: Latitude $lat, Longitude $lon, Bearing: $bearing")
                ShashlikMapApiHolder.shashlikMapApi?.setLatLonBearing(
                    lat = lat,
                    lon = lon,
                    bearing = bearing?.toFloat()
                )
            },
            onError = { errorMessage ->
                println("Failed to fetch location: $errorMessage")
            }
        ).also {
            it.startUpdatingLocation()
        }
    }

    UIKitView(
        factory = {
            MetalUIView()
        },
        modifier = Modifier.fillMaxSize(),
        // disable interactionMode to give a full control to Compose
        // otherwise some gesture may not work correctly
        properties = UIKitInteropProperties(
            interactionMode = null
        )
    )
}