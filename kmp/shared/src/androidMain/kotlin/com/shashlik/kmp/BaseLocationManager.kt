package com.shashlik.kmp

data class LocationData(val lat: Double, val lon: Double, val bearing: Float?)

abstract class BaseLocationManager(protected val callback: (LocationData) -> Unit) {
    abstract fun start()

    abstract fun stop()
}