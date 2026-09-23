package com.shashlik.kmp

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.unit.Dp
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

internal fun ComposeColor.toShashlikColor(): Color = Color(r = red, g = green, b = blue)

/**
 * Draws a convex polygon overlay on the map.
 *
 * @param center The geographic center point of the polygon.
 * @param radius The distance from the center to each vertex as a [Dp] value.
 * @param sides The number of sides (vertices) of the polygon. Must be at least 3.
 * @param color The color used to fill the polygon.
 */
@Composable
fun ConvexPolygon(
    center: Point,
    radius: Dp,
    sides: Int,
    color: ComposeColor,
) {
    val radiusDp = radius.value.toDouble()
    val points = (0 until sides).map { i ->
        val angle = 2.0 * PI * i / sides
        Point(
            x = cos(angle) * radiusDp,
            y = sin(angle) * radiusDp,
        )
    }
    ShashlikShape(
        points = points,
        anchor = center,
        type = ShapeType.Polygon,
        color = color.toShashlikColor()
    )
}

/**
 * Draws a line overlay connecting a series of geographic points on the map.
 *
 * @param points The list of geographic points defining the path of the line in Mercator coordinates.
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
        anchor = null,
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
 * @param points The points defining the shape. If [anchor] is null, these points are
 * treated as Mercator coordinates. If [anchor] is provided, these points are treated
 * as relative offset points in dp from the anchor.
 * @param anchor The optional geographic anchor point for the shape. If null, [points] are
 * Mercator coordinates; otherwise [points] are relative offset points from this anchor.
 * @param type The type of shape to render (e.g., POLYGON, LINE).
 * @param color The color of the shape.
 */
@Composable
fun ShashlikShape(
    points: List<Point>,
    anchor: Point?,
    type: ShapeType,
    color: Color
) {
    DisposableEffect(points, type, color) {
        var shapeId: String? = null

        val job = CoroutineScope(Dispatchers.Main).launch {
            val api = awaitApi()
            shapeId = api.addOverlayShape(points, anchor,type, color)
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