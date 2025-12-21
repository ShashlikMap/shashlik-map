package com.shashlik.kmp

import android.annotation.SuppressLint
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleStartEffect
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.rememberMultiplePermissionsState
import timber.log.Timber
import timber.log.Timber.DebugTree

fun shashlikMapInit() {
    Timber.plant(DebugTree())
}

@OptIn(ExperimentalPermissionsApi::class)
@SuppressLint("MissingPermission")
@Composable
actual fun ShashlikMap(onLongTap: (x: Float, y: Float) -> Unit) {
    val locationPermissionState = rememberMultiplePermissionsState(
        listOf(
            android.Manifest.permission.ACCESS_FINE_LOCATION,
            android.Manifest.permission.ACCESS_COARSE_LOCATION
        )
    )

    if (locationPermissionState.allPermissionsGranted) {
        ShashlikMapComp(onLongTap)
    } else {
        LaunchedEffect(Unit) {
            locationPermissionState.launchMultiplePermissionRequest()
        }
    }
}

@SuppressLint("MissingPermission")
@Composable
private fun ShashlikMapComp(onLongTap: (x: Float, y: Float) -> Unit) {
    val ctx = LocalContext.current
    val locationManager = remember {
        SimpleLocationManager(ctx) {
            ShashlikMapApiHolder.shashlikMapApi?.setLatLonBearing(it.lat, it.lon, it.bearing)
        }
    }
    LifecycleStartEffect(Unit) {
        Timber.d("onStart")
        locationManager.start()

        onStopOrDispose {
            Timber.d( "onStop")
            locationManager.stop()
        }
    }

    AndroidView(
        factory = { ctx ->
            WGPUTextureView(context = ctx).also {
                it.onLongTap = onLongTap
            }
        },
        modifier = Modifier.fillMaxSize()
    )
}