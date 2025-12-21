package com.shashlik.kmp

import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.util.Log

object TempLocationManager {
    var lastLocation: Location? = null

    var map: WGPUSurfaceView? = null

    val locationListener = LocationListener { location ->
        lastLocation = location
        val latitude: Double = location.latitude
        val longitude: Double = location.longitude
        Log.d("kiol", "latitude = $latitude, longitude = $longitude")
        Log.d("kiol", "hasAccuracy = ${location.hasAccuracy()}, accuracy = ${location.accuracy}")
        Log.d("kiol", "hasBearing = ${location.hasBearing()}, bearing = ${location.bearing}")
        Log.d(
            "kiol",
            "hasBearingAccuracy = ${location.hasBearingAccuracy()}, bearingAccuracyDegrees = ${location.bearingAccuracyDegrees}"
        )
        Log.d("kiol", "hasAltitude = ${location.hasAltitude()}, altitude = ${location.altitude}")
        val bearing: Float? = if (location.hasBearing()) location.bearing else null
        ShashlikMapApiHolder.shashlikMapApi?.setLatLonBearing(latitude, longitude, bearing)
    }

    lateinit var locationService: LocationManager
}