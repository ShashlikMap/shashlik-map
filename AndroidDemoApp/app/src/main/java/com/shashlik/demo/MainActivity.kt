package com.shashlik.demo

import android.Manifest
import android.content.Context
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Bundle
import android.util.Log
import android.view.Surface
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.annotation.RequiresPermission
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.lifecycleScope
import com.shashlik.demo.ui.theme.MyApplicationTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class RB {

    init {
        System.loadLibrary("ffi_run")
    }

    external fun createShashlikMapApi(surface: Surface, isEmulator: Boolean, tilesDb: String): Long
}

class MainActivity : ComponentActivity() {

    private var lastLocation: Location? = null

    private val locationListener = LocationListener { location ->
        lastLocation = location
        val latitude: Double = location.latitude
        val longitude: Double = location.longitude
        Log.d("kiol", "latitude = $latitude, longitude = $longitude")
        Log.d("kiol", "hasAccuracy = ${location.hasAccuracy()}, accuracy = ${location.accuracy}")
        Log.d("kiol", "hasBearing = ${location.hasBearing()}, bearing = ${location.bearing}")
        Log.d("kiol", "hasBearingAccuracy = ${location.hasBearingAccuracy()}, bearingAccuracyDegrees = ${location.bearingAccuracyDegrees}")
        val bearing: Float? = if(location.hasBearing()) location.bearing else null
        map?.shashlikMapApi?.setLatLonBearing(latitude, longitude, bearing)
    }

    lateinit var locationService: LocationManager

    private var map: WGPUSurfaceView? = null

    @RequiresPermission(allOf = [Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION])
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        locationService = getSystemService(Context.LOCATION_SERVICE) as LocationManager

        enableEdgeToEdge()
        setContent {
            MyApplicationTheme {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(24.dp)
                ) {
                    AndroidView(
                        factory = { ctx ->
                            val sv = WGPUSurfaceView(context = ctx)
                            map = sv
                            this@MainActivity.lifecycleScope.launch {
                                delay(1000)
                                lastLocation?.let {
                                    this@MainActivity.locationListener.onLocationChanged(it)
                                }
                            }
                            sv
                        },
                        modifier = Modifier
                            .fillMaxSize()
                    )

                    Row(
                        modifier = Modifier.align(Alignment.BottomCenter),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        var checkedState by remember { mutableStateOf(true) }
                        Checkbox(
                            checkedState,
                            onCheckedChange = {
                                map?.shashlikMapApi?.setCamFollowMode(it)
                                checkedState = it
                            })
                        Text("Camera Mode")
                    }
                }
            }
        }
    }

    @RequiresPermission(allOf = [Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION])
    override fun onStart() {
        super.onStart()
        Log.d("kiol", "onStart")
        locationService.getLastKnownLocation(LocationManager.GPS_PROVIDER)?.let {
            Log.d("kiol", "getLastKnownLocation $it")
            locationListener.onLocationChanged(it)
        }

        locationService.requestLocationUpdates(
            LocationManager.GPS_PROVIDER,
            1000L,
            2f,
            locationListener
        )

    }

    override fun onStop() {
        super.onStop()
        Log.d("kiol", "onStop")
        locationService.removeUpdates(locationListener)
    }
}

@Composable
fun Greeting(name: String, modifier: Modifier = Modifier) {
    Text(
        text = "Hello $name!",
        modifier = modifier
    )
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    MyApplicationTheme {
        Greeting("Android")
    }
}