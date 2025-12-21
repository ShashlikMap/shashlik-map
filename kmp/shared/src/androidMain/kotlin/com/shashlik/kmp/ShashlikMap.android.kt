package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.location.LocationManager
import android.util.Log
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleStartEffect
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.rememberMultiplePermissionsState
import kotlinx.coroutines.delay

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
    LifecycleStartEffect(Unit) {
        Log.d("kiol", "onStart")
        TempLocationManager.locationService.getLastKnownLocation(LocationManager.GPS_PROVIDER)
            ?.let {
                Log.d("kiol", "getLastKnownLocation $it")
                TempLocationManager.locationListener.onLocationChanged(it)
            }

        TempLocationManager.locationService.requestLocationUpdates(
            LocationManager.GPS_PROVIDER,
            1000L,
            2f,
            TempLocationManager.locationListener
        )


        onStopOrDispose {
            Log.d("kiol", "onStop")
            TempLocationManager.locationService.removeUpdates(TempLocationManager.locationListener)
        }
    }
    LaunchedEffect(Unit) {
        delay(1000)
        TempLocationManager.lastLocation?.let {
            TempLocationManager.locationListener.onLocationChanged(it)
        }
    }
    AndroidView(
        factory = { ctx ->
            TempLocationManager.locationService =
                ctx.getSystemService(Context.LOCATION_SERVICE) as LocationManager
            val sv = WGPUSurfaceView(context = ctx)
            sv.onLongTap = onLongTap
            TempLocationManager.map = sv
            sv
        },
        modifier = Modifier.fillMaxSize()
    )
}