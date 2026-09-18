package com.shashlik.kmp

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.listSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue

/**
 * Creates and [remember]s a [LocationState] that survives configuration changes.
 *
 * @param latitude Initial latitude.
 * @param longitude Initial longitude.
 * @param bearing Initial bearing.
 */
@Composable
public fun rememberLocationState(
    latitude: Double = 0.0,
    longitude: Double = 0.0,
    bearing: Float? = null
): LocationState {
    return rememberSaveable(saver = LocationState.Saver) {
        LocationState(latitude, longitude, bearing)
    }
}

/**
 * A state object that can be hoisted to control and observe the map's location and bearing.
 *
 * In most cases, this will be created via [rememberLocationState].
 */
@Stable
class LocationState(
    latitude: Double = 0.0,
    longitude: Double = 0.0,
    bearing: Float? = null
) {
    /**
     * Current latitude of the location marker.
     */
    var latitude by mutableStateOf(latitude)

    /**
     * Current longitude of the location marker.
     */
    var longitude by mutableStateOf(longitude)

    /**
     * Current bearing of the location marker in degrees, or null if not available.
     */
    var bearing by mutableStateOf(bearing)

    companion object {
        /**
         * The default [Saver] implementation for [LocationState].
         */
        val Saver: Saver<LocationState, *> = listSaver(
            save = { listOf(it.latitude, it.longitude, it.bearing) },
            restore = {
                LocationState(
                    latitude = it[0] as Double,
                    longitude = it[1] as Double,
                    bearing = it[2] as Float?
                )
            }
        )
    }
}