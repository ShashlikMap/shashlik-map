package com.shashlik.kmp

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculateCentroidSize
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.util.fastAny
import androidx.compose.ui.util.fastForEach
import kotlinx.coroutines.delay
import uniffi.ffi_run.ShashlikMapApi
import kotlin.math.abs
import kotlin.time.Duration.Companion.milliseconds

/**
 * Indicates whether the current build is a debug build.
 */
expect val isDebugBuild: Boolean

private const val PITCH_SCROLL_DIVISOR = 10.0f

/**
 * Slightly modified version of PointerInputScope.detectTransformGestures
 */
private suspend fun PointerInputScope.detectTwoFingersScrollZoom(
    onGesture: (centroid: Offset, scroll: Float, zoom: Float) -> Unit,
) {
    awaitEachGesture {
        var zoom = 1f
        var pan = Offset.Zero
        var pastTouchSlop = false
        val touchSlop = viewConfiguration.touchSlop
        var lockedToPan = false

        awaitFirstDown(requireUnconsumed = false)
        do {
            val event = awaitPointerEvent()
            val canceled = event.changes.fastAny { it.isConsumed }
            if (!canceled && event.changes.size == 2) {
                val zoomChange = event.calculateZoom()
                val panChange = event.calculatePan()

                if (!pastTouchSlop) {
                    zoom *= zoomChange
                    pan += panChange

                    val centroidSize = event.calculateCentroidSize(useCurrent = false)
                    val zoomMotion = abs(1 - zoom) * centroidSize
                    val panMotion = pan.getDistance()

                    if (zoomMotion > touchSlop) {
                        lockedToPan = false
                        pastTouchSlop = true
                    } else if (panMotion > touchSlop) {
                        lockedToPan = true
                        pastTouchSlop = true
                    }
                }

                if (pastTouchSlop) {
                    val centroid = event.calculateCentroid(useCurrent = false)
                    val effectiveZoom = if (lockedToPan) 1.0f else zoomChange
                    val effectivePan = if (lockedToPan) panChange else Offset.Zero
                    onGesture(centroid, effectivePan.y / PITCH_SCROLL_DIVISOR, effectiveZoom)
                    event.changes.fastForEach {
                        if (it.positionChanged()) {
                            it.consume()
                        }
                    }
                }
            }
        } while (!canceled && event.changes.fastAny { it.pressed })
    }
}

/**
 * Extension modifier that attaches touch gesture detectors to the map view container.
 *
 * It handles single-finger pan gestures (map dragging) as well as two-finger gestures
 * for zooming and pitching the camera view using the low-level map API.
 *
 * @param onGesture Optional callback invoked whenever a user gesture is detected (e.g., to disable camera follow mode).
 * @return A [Modifier] with map gesture input processing attached.
 */
fun Modifier.mapGestures(onGesture: () -> Unit = {}): Modifier = pointerInput(Unit) {
    detectTransformGestures { _, pan, _, _ ->
        val panX = pan.x
        val panY = pan.y
        onGesture()
        ShashlikMapApiHolder.shashlikMapApi?.panDelta(-panX, -panY)
    }
}.pointerInput(Unit) {
    detectTwoFingersScrollZoom { centroid, scroll, zoom ->
        onGesture()
        if (zoom != 1.0f) {
            ShashlikMapApiHolder.shashlikMapApi?.zoomDelta(
                zoom, centroid.x, centroid.y
            )
        } else if (scroll != 0.0f) {
            ShashlikMapApiHolder.shashlikMapApi?.pitchDelta(scroll)
        }
    }
}

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
    modifier: Modifier = Modifier,
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
    Box(modifier = modifier
        .background(Color(0xFFF4F3F0))) {
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

