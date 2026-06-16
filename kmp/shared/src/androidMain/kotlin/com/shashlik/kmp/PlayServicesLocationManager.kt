package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.os.Looper
import com.google.android.gms.location.FusedLocationProviderClient
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationResult
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority

class PlayServicesLocationManager(
    context: Context,
    callback: (LocationData) -> Unit
) : BaseLocationManager(callback) {

    private val fusedLocationClient: FusedLocationProviderClient =
        LocationServices.getFusedLocationProviderClient(context)

    // Configure location request settings
    private val locationRequest: LocationRequest = LocationRequest.Builder(
        Priority.PRIORITY_HIGH_ACCURACY,
        1000L // Interval in milliseconds
    ).apply {
        setMinUpdateIntervalMillis(1000L) // Fastest interval
    }.build()

    // Handle incoming location updates
    private val locationCallback = object : LocationCallback() {
        override fun onLocationResult(locationResult: LocationResult) {
            val lastLocation = locationResult.lastLocation ?: return

            // Map Android Location to your custom LocationData
            val data = LocationData(
                lat = lastLocation.latitude,
                lon = lastLocation.longitude,
                bearing = if (lastLocation.hasBearing()) lastLocation.bearing else null
            )
            callback(data)
        }
    }

    /**
     * Starts receiving location updates.
     * Ensure ACCESS_FINE_LOCATION permission is granted before calling.
     */
    @SuppressLint("MissingPermission")
    override fun start() {
        fusedLocationClient.requestLocationUpdates(
            locationRequest,
            locationCallback,
            Looper.getMainLooper()
        )
    }

    /**
     * Stops receiving location updates to save battery.
     * Call this in onStop() or onDestroy() of your LifecycleOwner.
     */
    override fun stop() {
        fusedLocationClient.removeLocationUpdates(locationCallback)
    }
}