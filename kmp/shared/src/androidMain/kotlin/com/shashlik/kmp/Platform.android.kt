package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.location.LocationManager
import android.util.Log
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleStartEffect
import kotlinx.coroutines.delay

@SuppressLint("MissingPermission")
@Composable
fun ShashlikMapComp() {
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
            modifier = Modifier.align(Alignment.BottomCenter),
            verticalAlignment = Alignment.CenterVertically
        ) {
            var checkedState by remember { mutableStateOf(true) }
            Checkbox(
                checkedState,
                onCheckedChange = {
                    TempLocationManager.map?.shashlikMapApi?.setCamFollowMode(it)
                    checkedState = it
                })
            Text("Camera Mode")
        }
    }

}