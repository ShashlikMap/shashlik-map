package com.shashlik.kmp

import android.content.Context
import android.location.LocationListener
import android.location.LocationManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelChildren
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import timber.log.Timber

data class LocationData(val lat: Double, val lon: Double, val bearing: Float?)
class SimpleLocationManager(context: Context, private val callback: (LocationData) -> Unit) {

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val locationService: LocationManager =
        context.getSystemService(Context.LOCATION_SERVICE) as LocationManager

    private val locationListener = LocationListener { location ->
        val latitude: Double = location.latitude
        val longitude: Double = location.longitude
        Timber.d("latitude = $latitude, longitude = $longitude")
        Timber.d("latitude = $latitude, longitude = $longitude")
        Timber.d("hasAccuracy = ${location.hasAccuracy()}, accuracy = ${location.accuracy}")
        Timber.d("hasBearing = ${location.hasBearing()}, bearing = ${location.bearing}")
        Timber.d(
            "hasBearingAccuracy = ${location.hasBearingAccuracy()}, bearingAccuracyDegrees = ${location.bearingAccuracyDegrees}"
        )
        Timber.d("hasAltitude = ${location.hasAltitude()}, altitude = ${location.altitude}")
        val bearing: Float? = if (location.hasBearing()) location.bearing else null
        callback(LocationData(latitude, longitude, bearing))
    }

    fun start() {
        locationService.requestLocationUpdates(
            LocationManager.GPS_PROVIDER,
            1000L,
            2f, locationListener
        )
        scope.launch {
            // wait a bit, otherwise locationService might not return last location yet(even though it has it)
            delay(500)
            locationService.getLastKnownLocation(LocationManager.GPS_PROVIDER)?.let {
                Timber.w("Cached location: $it")
                locationListener.onLocationChanged(it)
            }
        }
    }

    fun stop() {
        scope.coroutineContext[Job]?.cancelChildren()
        locationService.removeUpdates(locationListener)
    }
}