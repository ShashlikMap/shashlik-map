package com.shashlik.kmp

import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.useContents
import platform.CoreLocation.CLLocation
import platform.CoreLocation.CLLocationManager
import platform.CoreLocation.CLLocationManagerDelegateProtocol
import platform.CoreLocation.CLAuthorizationStatus
import platform.CoreLocation.kCLAuthorizationStatusNotDetermined
import platform.CoreLocation.kCLAuthorizationStatusRestricted
import platform.CoreLocation.kCLAuthorizationStatusDenied
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedWhenInUse
import platform.CoreLocation.kCLAuthorizationStatusAuthorizedAlways
import platform.Foundation.NSError
import platform.darwin.NSObject

class IOSLocationProvider(
    private val onLocationUpdated: (latitude: Double, longitude: Double, bearing: Double?) -> Unit,
    private val onError: (String) -> Unit
) {
    private val locationManager = CLLocationManager()

    // Keep a strong reference to the delegate so it isn't garbage collected
    private val delegate = LocationDelegate()

    init {
        locationManager.delegate = delegate
    }

    /**
     * Checks permission status and requests location updates.
     */
    fun startUpdatingLocation() {
        val status = locationManager.authorizationStatus()
        handleAuthorizationStatus(status)
    }

    /**
     * Safely stops location tracking to preserve battery life.
     */
    fun stopUpdatingLocation() {
        locationManager.stopUpdatingLocation()
    }

    private fun handleAuthorizationStatus(status: CLAuthorizationStatus) {
        when (status) {
            kCLAuthorizationStatusNotDetermined -> {
                // Request permission if never asked before
                locationManager.requestWhenInUseAuthorization()
            }

            kCLAuthorizationStatusRestricted, kCLAuthorizationStatusDenied -> {
                onError("Location permission denied or restricted by system settings.")
            }

            kCLAuthorizationStatusAuthorizedWhenInUse, kCLAuthorizationStatusAuthorizedAlways -> {
                // Permissions granted, safely start hardware GPS updates
                locationManager.startUpdatingLocation()
            }
        }
    }

    /**
     * Inner class implementing the iOS CLLocationManagerDelegate Protocol
     */
    private inner class LocationDelegate : NSObject(), CLLocationManagerDelegateProtocol {

        @OptIn(ExperimentalForeignApi::class)
        override fun locationManager(
            manager: CLLocationManager,
            didUpdateLocations: List<*>
        ) {
            val locations = didUpdateLocations as? List<CLLocation> ?: return
            val lastLocation = locations.lastOrNull() ?: return

            // Extract coordinates directly using Kotlin interop
            val coordinate = lastLocation.coordinate
            val latitude = coordinate.useContents { latitude }
            val longitude = coordinate.useContents { longitude }
            val course = lastLocation.course
            val bearing: Double? = if (course >= 0.0) {
                course
            } else {
                null
            }

            onLocationUpdated(latitude, longitude, bearing)
        }

        override fun locationManager(
            manager: CLLocationManager,
            didChangeAuthorizationStatus: CLAuthorizationStatus
        ) {
            // Triggered automatically when the user interacts with the system permission dialog
            handleAuthorizationStatus(didChangeAuthorizationStatus)
        }

        override fun locationManager(
            manager: CLLocationManager,
            didFailWithError: NSError
        ) {
            onError(didFailWithError.localizedDescription ?: "Unknown CoreLocation error")
        }
    }
}
