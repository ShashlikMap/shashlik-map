package com.shashlik.kmp

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import kotlinx.coroutines.delay
import uniffi.ffi_run.ShashlikMapApi
import kotlin.time.Duration.Companion.milliseconds

/**
 * Indicates whether the current build is a debug build.
 */
expect val isDebugBuild: Boolean

/**
 * The main entry point for the Shashlik Map component.
 *
 * This composable initializes the map engine and provides a container for map overlays.
 *
 * @param state The [LocationState] to control and observe the map's location.
 * @param withPuck When true, the map will display a "puck" (location marker) at the current location.
 * @param withAutoLocationEvent When true, the map will automatically track and display the user's location.
 * @param mvtTiles Whether to enable MVT (Mapbox Vector Tile) rendering.
 * @param followModeEnabled Whether the camera should follow the location puck automatically.
 * @param camAnimationEnabled Whether camera position updates should be animated.
 * @param puckAnimationEnabled Whether location puck updates should be animated.
 * @param content The content to be rendered on top of the map, typically map overlays like [ConvexPolygon] or [LineShape].
 */
@Composable
fun ShashlikMap(
    state: LocationState = rememberLocationState(),
    withPuck: Boolean = true,
    withAutoLocationEvent: Boolean = true,
    mvtTiles: Boolean = false,
    followModeEnabled: Boolean = true,
    camAnimationEnabled: Boolean = true,
    puckAnimationEnabled: Boolean = true,
    content: @Composable () -> Unit = {}
) {
    LaunchedEffect(mvtTiles, withPuck) {
        awaitApi().run {
            puckConfig(withPuck)
            setMvtTileset(mvtTiles)
        }
    }

    LaunchedEffect(followModeEnabled, camAnimationEnabled, puckAnimationEnabled) {
        awaitApi().run {
            setCamFollowMode(followModeEnabled)
            setAnimEnabled(camAnimationEnabled, puckAnimationEnabled)
        }
    }

    LaunchedEffect(state.latitude, state.longitude, state.bearing) {
        if(state.latitude != 0.0 || state.longitude != 0.0) {
            awaitApi().setLatLonBearing(state.latitude, state.longitude, state.bearing)
        }
    }

    // this is a ground color from the styles
    // TODO How to get this info here from the styles
    Box(modifier = Modifier.background(Color(0xFFF4F3F0))) {
        ShashlikMapSetup(state, withAutoLocationEvent)
        content()
    }
}

@Composable
internal expect fun ShashlikMapSetup(state: LocationState, withAutoLocationEvent: Boolean)


private const val API_WAIT_ATTEMPTS = 40
private const val API_POLL_MS = 50L
internal suspend fun awaitApi(): ShashlikMapApi {
    repeat(API_WAIT_ATTEMPTS) {
        ShashlikMapApiHolder.shashlikMapApi?.let { return it }
        delay(API_POLL_MS.milliseconds)
    }
    throw NullPointerException("shashlikMapApi never became available; nothing drawn")
}

/**
 * A global holder for the [ShashlikMapApi] instance.
 *
 * This provides access to the map's low-level API once it has been initialized.
 */
object ShashlikMapApiHolder {
    /**
     * The active [ShashlikMapApi] instance, or null if not yet initialized.
     */
    var shashlikMapApi by mutableStateOf<ShashlikMapApi?>(null)
}

