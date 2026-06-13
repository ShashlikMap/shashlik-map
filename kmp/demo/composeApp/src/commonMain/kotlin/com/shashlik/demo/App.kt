package com.shashlik.demo

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculateCentroidSize
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.fastAny
import androidx.compose.ui.util.fastForEach
import com.shashlik.kmp.ShashlikMap
import com.shashlik.kmp.ShashlikMapApiHolder
import com.shashlik.kmp.isDebugBuild
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.ffi_run.RouteCosting.AUTO
import uniffi.ffi_run.RouteCosting.MOTORBIKE
import uniffi.ffi_run.RouteCosting.PEDESTRIAN
import uniffi.ffi_run.RouteCosting.entries
import kotlin.math.abs

var routeCosting = mutableStateOf(AUTO)

/**
 * Slightly modified version of PointerInputScope.detectTransformGestures
 */
suspend fun PointerInputScope.detectTwoFingersScrollZoom(
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
                    onGesture(centroid, effectivePan.y, effectiveZoom)
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

@Composable
@Preview
fun App() {
    MaterialTheme {
        Box(modifier = Modifier.fillMaxSize().pointerInput(Unit) {
            detectTapGestures(onLongPress = { offset ->
                ShashlikMapApiHolder.shashlikMapApi?.calculateRoute(
                    offset.x, offset.y, routeCosting.value
                )
            })
        }.pointerInput(Unit) {
            detectTransformGestures { _, pan, _, _ ->
                val panX = pan.x / 15.0f
                val panY = pan.y / 15.0f
                ShashlikMapApiHolder.shashlikMapApi?.panDelta(-panX, -panY)
            }
        }
            .pointerInput(Unit) {
                detectTwoFingersScrollZoom { centroid, scroll, zoom ->
                    if (zoom != 1.0f) {
                        // zoom is the scaleFactor relative to the previous frame (e.g., 1.05f)
                        val zoomDelta = (zoom - 1.0f) * 150.0f

                        ShashlikMapApiHolder.shashlikMapApi?.zoomDelta(
                            zoomDelta, centroid.x, centroid.y
                        )
                    } else if (scroll != 0.0f) {
                        ShashlikMapApiHolder.shashlikMapApi?.pitchDelta(scroll / 10.0f)
                    }
                }
            }
        ) {
            ShashlikMap()
            Row(
                modifier = Modifier.fillMaxWidth().align(Alignment.BottomCenter)
                    .background(Color(0, 0, 0, 120)).padding(16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Button(onClick = {
                    routeCosting.value = ((routeCosting.value.ordinal + 1) % entries.size).let {
                        entries[it]
                    }
                }) {
                    when (routeCosting.value) {
                        AUTO -> Text("Auto")
                        PEDESTRIAN -> Text("Pedestrian")
                        MOTORBIKE -> Text("Motorbike")
                    }
                }
                Spacer(modifier = Modifier.width(8.dp))
                Column {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        var checkedState by remember { mutableStateOf(true) }
                        Checkbox(
                            checkedState, onCheckedChange = {
                                ShashlikMapApiHolder.shashlikMapApi?.setCamFollowMode(it)
                                checkedState = it
                            })
                        Text("Camera Mode")
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        var ssaoCheckedState by remember { mutableStateOf(false) }
                        Checkbox(
                            ssaoCheckedState, onCheckedChange = {
                                ShashlikMapApiHolder.shashlikMapApi?.setSsaoMode(it)
                                ssaoCheckedState = it
                            })
                        Text("SSAO")

                        var previewCheckedState by remember { mutableStateOf(false) }
                        Checkbox(
                            previewCheckedState, onCheckedChange = {
                                ShashlikMapApiHolder.shashlikMapApi?.setPreviewEnabled(it)
                                previewCheckedState = it
                            })
                        Text("Preview")
                    }
                }

            }
            Text(
                "Build:${if (isDebugBuild) "Debug" else "Release"}",
                modifier = Modifier.align(Alignment.BottomEnd).padding(bottom = 8.dp, end = 8.dp)
            )
        }
    }
}