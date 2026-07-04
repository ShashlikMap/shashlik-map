package com.shashlik.kmp

import android.annotation.SuppressLint
import android.app.Activity
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleStartEffect
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.rememberMultiplePermissionsState
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.shashlik.kmp.ShashlikMapApiHolder.shashlikMapApi
import com.shashlik.kmp.shared.BuildConfig
import timber.log.Timber
import timber.log.Timber.DebugTree

@Suppress("KotlinConstantConditions")
actual val isDebugBuild: Boolean = BuildConfig.DEBUG

fun shashlikMapInit() {
    Timber.plant(DebugTree())
}

@OptIn(ExperimentalPermissionsApi::class)
@SuppressLint("MissingPermission")
@Composable
actual fun ShashlikMap() {
    if (LocalInspectionMode.current) {
        Box(modifier = Modifier.fillMaxSize().background(Color.DarkGray)) {
            Text("ShashlikMap Preview", color = Color.White, modifier = Modifier.align(Alignment.Center))
        }
        return
    }

    val locationPermissionState = rememberMultiplePermissionsState(
        listOf(
            android.Manifest.permission.ACCESS_FINE_LOCATION,
            android.Manifest.permission.ACCESS_COARSE_LOCATION
        )
    )

    if (locationPermissionState.allPermissionsGranted) {
        ShashlikMapComp()
    } else {
        LaunchedEffect(Unit) {
            locationPermissionState.launchMultiplePermissionRequest()
        }
    }
}

@SuppressLint("MissingPermission")
@Composable
private fun ShashlikMapComp() {
    val ctx = LocalContext.current
    val locationManager = remember {
        val locationCallback: (LocationData) -> Unit = {
            shashlikMapApi?.setLatLonBearing(it.lat, it.lon, it.bearing)
        }
        if (checkPlayServices(ctx as Activity)) {
            Timber.i("PlayServicesLocationManager is used")
            PlayServicesLocationManager(ctx, locationCallback)
        } else {
            Timber.i("AOSP pure location manager is used")
            SimpleLocationManager(ctx, locationCallback)
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
            WGPUTextureView(context = ctx)
        },
        modifier = Modifier.fillMaxSize()
    )
}

private fun checkPlayServices(activity: Activity): Boolean {
    val apiAvailability = GoogleApiAvailability.getInstance()
    val resultCode = apiAvailability.isGooglePlayServicesAvailable(activity)

    if (resultCode == ConnectionResult.SUCCESS) {
        Timber.d("Google Play Services is available.")
        return true
    }

    Timber.e("This device is not supported by Google Play Services(or outdated!).")
    return false
}