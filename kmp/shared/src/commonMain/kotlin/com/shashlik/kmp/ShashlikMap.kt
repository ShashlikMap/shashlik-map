package com.shashlik.kmp

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import kotlinx.coroutines.delay
import uniffi.ffi_run.ShashlikMapApi
import kotlin.time.Duration.Companion.milliseconds

expect val isDebugBuild: Boolean

/**
 * The main entry point for the Shashlik Map component.
 *
 * This composable initializes the map engine and provides a container for map overlays.
 *
 * @param withAutoLocationEvent When true, the map will automatically track and display the user's location.
 * @param mvtTiles Whether to enable MVT (Mapbox Vector Tile) rendering.
 * @param content The content to be rendered on top of the map, typically map overlays like [ConvexPolygon] or [LineShape].
 */
@Composable
fun ShashlikMap(
    withAutoLocationEvent: Boolean = true,
    mvtTiles: Boolean,
    content: @Composable () -> Unit = {}
) {
    LaunchedEffect(mvtTiles) {
        awaitApi().setMvtTileset(mvtTiles)
    }
    ShashlikMapSetup(withAutoLocationEvent)
    content()
}

@Composable
internal expect fun ShashlikMapSetup(withAutoLocationEvent: Boolean)


private const val API_WAIT_ATTEMPTS = 40
private const val API_POLL_MS = 50L
internal suspend fun awaitApi(): ShashlikMapApi {
    repeat(API_WAIT_ATTEMPTS) {
        ShashlikMapApiHolder.shashlikMapApi?.let { return it }
        delay(API_POLL_MS.milliseconds)
    }
    throw NullPointerException("shashlikMapApi never became available; nothing drawn")
}

object ShashlikMapApiHolder {
    var shashlikMapApi: ShashlikMapApi? = null
}

