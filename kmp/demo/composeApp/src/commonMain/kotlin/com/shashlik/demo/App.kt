package com.shashlik.demo

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.shashlik.kmp.ConvexPolygon
import com.shashlik.kmp.LineShape
import com.shashlik.kmp.ShashlikMap
import com.shashlik.kmp.ShashlikMapApiHolder
import com.shashlik.kmp.ShashlikShape
import com.shashlik.kmp.isDebugBuild
import com.shashlik.kmp.mapGestures
import kotlinx.coroutines.delay
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.ffi_run.Point
import uniffi.ffi_run.RouteCosting.AUTO
import uniffi.ffi_run.RouteCosting.MOTORBIKE
import uniffi.ffi_run.RouteCosting.PEDESTRIAN
import uniffi.ffi_run.RouteCosting.entries
import uniffi.ffi_run.ShapeType
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.random.Random
import kotlin.time.Duration.Companion.milliseconds

var routeCosting = mutableStateOf(AUTO)

private const val TOKYO_CENTER_X = 139.757080078125
private const val TOKYO_CENTER_Y = 35.68798828125
private const val TOKYO_MAX_OFFSET = 0.01
private const val EXTENDED_CONTROLS = false

@Composable
@Preview
fun App() {
    MaterialTheme {
        Box(modifier = Modifier
            .fillMaxSize()
            .pointerInput(Unit) {
                detectTapGestures(onLongPress = { offset ->
                    ShashlikMapApiHolder.shashlikMapApi?.calculateRoute(
                        offset.x, offset.y, routeCosting.value
                    )
                })
            }
        ) {
            var mvtCheckedState by remember { mutableStateOf(true) }
            var camFollowModeState by remember { mutableStateOf(true) }
            val shapes = remember { mutableStateListOf(generateRandomShapeAroundTokyo()) }

            var movingPoint by remember { mutableStateOf(Point(TOKYO_CENTER_X, TOKYO_CENTER_Y)) }
            LaunchedEffect(Unit) {
                var angle = 0.0
                while (true) {
                    delay(16.milliseconds)
                    angle += 0.005
                    val x = TOKYO_CENTER_X + (TOKYO_MAX_OFFSET * 0.8) * cos(angle)
                    val y = TOKYO_CENTER_Y + (TOKYO_MAX_OFFSET * 0.8) * sin(angle * 0.7)
                    movingPoint = Point(x, y)
                }
            }

            ShashlikMap(
                modifier = Modifier.mapGestures(),
                withAutoLocationEvent = true,
                withPuck = true,
                followModeEnabled = camFollowModeState,
                camAnimationEnabled = true,
                puckAnimationEnabled = true,
                mvtTiles = mvtCheckedState
            ) {
                ConvexPolygon(
                    center = movingPoint,
                    radius = 6.dp,
                    sides = 5,
                    color = Color.White
                )
                ConvexPolygon(
                    center = movingPoint,
                    radius = 5.dp,
                    sides = 5,
                    color = Color.Blue
                )
                shapes.forEach { shape ->
                    when (shape) {
                        is RandomShape.Line -> {
                            LineShape(shape.points, shape.color, width = shape.width)
                        }

                        is RandomShape.Convex -> {
                            ConvexPolygon(shape.center, shape.radius, shape.sides, shape.color)
                        }

                        is RandomShape.Polygon -> {
                            ShashlikShape(
                                points = shape.points,
                                anchor = null,
                                type = ShapeType.Polygon,
                                color = shape.color
                            )
                        }
                    }
                }
            }
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.BottomCenter)
                    .background(Color(0, 0, 0, 120))
                    .padding(16.dp),
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
                Column(verticalArrangement = Arrangement.Center) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(
                            camFollowModeState, onCheckedChange = {
                                camFollowModeState = it
                            })
                        Text("Camera Mode")

                        Checkbox(
                            mvtCheckedState, onCheckedChange = {
                                mvtCheckedState = it
                            })
                        Text("MVT")

                        Button({
                            if (shapes.size > 5) {
                                shapes.clear()
                            }
                            shapes.add(generateRandomShapeAroundTokyo())
                        }) {
                            Text("Shp")
                        }
                    }
                    if(EXTENDED_CONTROLS) {
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
                    } else {
                        Spacer(modifier = Modifier.height(32.dp))
                    }
                }

            }
            Text(
                "Build:${if (isDebugBuild) "Debug" else "Release"}",
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(bottom = 8.dp, end = 8.dp)
            )
        }
    }
}

private sealed interface RandomShape {
    data class Line(val points: List<Point>, val color: Color, val width: Float) : RandomShape
    data class Convex(val center: Point, val radius: Dp, val sides: Int, val color: Color) : RandomShape
    data class Polygon(val points: List<Point>, val color: Color) : RandomShape
}

private fun generateRandomShapeAroundTokyo(): RandomShape {
    val centerX = TOKYO_CENTER_X
    val centerY = TOKYO_CENTER_Y
    val maxOffset = TOKYO_MAX_OFFSET

    val randomColor = Color(
        red = Random.nextFloat(),
        green = Random.nextFloat(),
        blue = Random.nextFloat()
    )

    return when (Random.nextInt(3)) {
        0 -> {
            val lineWidth = 15f * Random.nextFloat()
            val points = (0 until 5).map {
                Point(
                    x = centerX + Random.nextDouble(-maxOffset, maxOffset),
                    y = centerY + Random.nextDouble(-maxOffset, maxOffset)
                )
            }
            RandomShape.Line(points, randomColor, lineWidth)
        }

        1 -> {
            val center = Point(
                x = centerX + Random.nextDouble(-maxOffset, maxOffset),
                y = centerY + Random.nextDouble(-maxOffset, maxOffset)
            )
            val radius = Random.nextInt(4, 15).dp
            val sides = Random.nextInt(3, 8)
            RandomShape.Convex(center, radius, sides, randomColor)
        }

        else -> {
            val shapeCenterX = centerX + Random.nextDouble(-maxOffset / 2, maxOffset / 2)
            val shapeCenterY = centerY + Random.nextDouble(-maxOffset / 2, maxOffset / 2)
            val numVertices = Random.nextInt(3, 7)
            val angles = DoubleArray(numVertices) { Random.nextDouble(0.0, 2 * PI) }.apply { sort() }
            val points = angles.map { angle ->
                val radius = Random.nextDouble(maxOffset * 0.5, maxOffset)
                Point(
                    x = shapeCenterX + radius * cos(angle),
                    y = shapeCenterY + radius * sin(angle)
                )
            }
            RandomShape.Polygon(points, randomColor)
        }
    }
}