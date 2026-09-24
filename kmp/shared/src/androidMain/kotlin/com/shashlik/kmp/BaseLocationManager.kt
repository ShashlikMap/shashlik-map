package com.shashlik.kmp

internal data class LocationData(val lat: Double, val lon: Double, val bearing: Float?)

internal abstract class BaseLocationManager(protected val callback: (LocationData) -> Unit) {
    abstract fun start()

    abstract fun stop()
}