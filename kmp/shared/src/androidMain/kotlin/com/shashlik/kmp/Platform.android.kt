package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.location.LocationManager
import android.util.Log
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleStartEffect
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.rememberMultiplePermissionsState
import kotlinx.coroutines.delay
import uniffi.ffi_run.RouteCosting

@OptIn(ExperimentalPermissionsApi::class)
@SuppressLint("MissingPermission")
@Composable
fun ShashlikMap() {
    // Camera permission state
    val cameraPermissionState = rememberMultiplePermissionsState(
        listOf(
            android.Manifest.permission.ACCESS_FINE_LOCATION,
            android.Manifest.permission.ACCESS_COARSE_LOCATION
        )
    )

    if (cameraPermissionState.allPermissionsGranted) {
        ShashlikMapComp()
    } else {
        LaunchedEffect(Unit) {
            cameraPermissionState.launchMultiplePermissionRequest()
        }
    }
}

@SuppressLint("MissingPermission")
@Composable
private fun ShashlikMapComp() {
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
    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(8.dp)
    ) {
        AndroidView(
            factory = { ctx ->
                TempLocationManager.locationService =
                    ctx.getSystemService(Context.LOCATION_SERVICE) as LocationManager
                val sv = WGPUSurfaceView(context = ctx)
                TempLocationManager.map = sv
                sv
            },
            modifier = Modifier.fillMaxSize()
        )
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .align(Alignment.BottomCenter)
                .background(Color(0, 0, 0, 150))
                .padding(horizontal = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Button(onClick = {
                if (routeCosting.value == RouteCosting.MOTORBIKE) {
                    routeCosting.value = RouteCosting.PEDESTRIAN
                } else {
                    routeCosting.value = RouteCosting.MOTORBIKE
                }
            }) {
                if (routeCosting.value == RouteCosting.MOTORBIKE) {
                    Text("Motorbike")
                } else {
                    Text("Pedestrian")
                }
            }
            Spacer(modifier = Modifier.width(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                var checkedState by remember { mutableStateOf(true) }
                Checkbox(
                    checkedState, onCheckedChange = {
                        TempLocationManager.map?.shashlikMapApi?.setCamFollowMode(it)
                        checkedState = it
                    })
                Text("Camera Mode")
            }
        }
    }
}