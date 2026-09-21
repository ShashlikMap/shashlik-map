package com.shashlik.kmp

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import uniffi.ffi_run.Color
import uniffi.ffi_run.Point
import uniffi.ffi_run.ShapeType
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import androidx.compose.ui.graphics.Color as ComposeColor

private const val DEG_TO_RAD = PI / 180.0
private const val METERS_PER_DEGREE_LAT = 111_320.0
private const val MIN_COS_LAT = 0.01

internal fun ComposeColor.toShashlikColor(): Color = Color(r = red, g = green, b = blue)

/**
 * Draws a convex polygon overlay on the map.
 *
 * @param center The geographic center point of the polygon.
 * @param radiusMeters The distance from the center to each vertex in meters.
 * @param sides The number of sides (vertices) of the polygon. Must be at least 3.
 * @param color The color used to fill the polygon.
 */
@Composable
fun ConvexPolygon(
    center: Point,
    radiusMeters: Double,
    sides: Int,
    color: ComposeColor,
) {
    val dLat = radiusMeters / METERS_PER_DEGREE_LAT
    val cosLat = cos(center.y * DEG_TO_RAD).coerceAtLeast(MIN_COS_LAT)
    val dLon = dLat / cosLat
    val points = (0 until sides).map { i ->
        val angle = 2.0 * PI * i / sides
        Point(
            x = center.x + dLon * cos(angle),
            y = center.y + dLat * sin(angle),
        )
    }
    ShashlikShape(points, ShapeType.Polygon, color.toShashlikColor())
}

/**
 * Draws a line overlay connecting a series of geographic points on the map.
 *
 * @param points The list of geographic points defining the path of the line.
 * @param color The color of the line.
 * @param width The width of the line. Note: this is an abstract unit at this moment;
 * a proper unit will be provided in a future update.
 */
@Composable
fun LineShape(points: List<Point>, color: ComposeColor, width: Float = 1f) {
    if (points.size < 2 || points.distinct().size < 2) {
        return
    }
    ShashlikShape(
        points = points,
        type = ShapeType.Line(width),
        color = color.toShashlikColor(),
    )
}

/**
 * A low-level component for rendering custom shapes on the map.
 *
 * It manages the lifecycle of a shape overlay, adding it to the map when entered
 * and removing it when disposed.
 *
 * @param points The geographic points defining the shape.
 * @param type The type of shape to render (e.g., POLYGON, LINE).
 * @param color The color of the shape.
 */
@Composable
fun ShashlikShape(
    points: List<Point>,
    type: ShapeType,
    color: Color
) {
    DisposableEffect(points, type, color) {
        var shapeId: String? = null

        val job = CoroutineScope(Dispatchers.Main).launch {
            val api = awaitApi()
            shapeId = api.addOverlayShape(points, type, color)
        }

        onDispose {
            job.cancel()
            CoroutineScope(Dispatchers.Main).launch {
                shapeId?.let { id ->
                    val api = awaitApi()
                    api.removeShape(id)
                }
            }
        }
    }
}

val ShapeType.Line.width: Float
    get() {
        return this.v1 ?: 1f
    }