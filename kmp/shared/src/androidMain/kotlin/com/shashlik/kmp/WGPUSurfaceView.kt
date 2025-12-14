package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.PixelFormat
import android.os.Build
import android.os.Parcelable
import android.util.AttributeSet
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.runtime.mutableStateOf
import uniffi.ffi_run.RouteCosting
import uniffi.ffi_run.ShashlikMapApi
import uniffi.ffi_run.toPointer

var routeCosting = mutableStateOf(RouteCosting.MOTORBIKE)

@SuppressLint("ClickableViewAccessibility")
class WGPUSurfaceView : SurfaceView, SurfaceHolder.Callback2 {

    private val scaleListener = object : ScaleGestureDetector.SimpleOnScaleGestureListener() {

        override fun onScale(detector: ScaleGestureDetector): Boolean {
            shashlikMapApi?.zoomDelta(
                (detector.scaleFactor - 1.0f) * 150.0f,
                detector.focusX,
                detector.focusY
            )
            return true
        }
    }

    private val mScaleDetector = ScaleGestureDetector(context, scaleListener)

    private val gestureListener = object : GestureDetector.SimpleOnGestureListener() {

        override fun onScroll(
            e1: MotionEvent?,
            e2: MotionEvent,
            distanceX: Float,
            distanceY: Float
        ): Boolean {
            if (e2.pointerCount == 2) {
                shashlikMapApi?.pitchDelta(-distanceY / 10.0f)
            } else {
                shashlikMapApi?.panDelta(distanceX / 15.0f, distanceY / 15.0f)
            }

            return super.onScroll(e1, e2, distanceX, distanceY)
        }

        override fun onLongPress(e: MotionEvent) {
            super.onLongPress(e)
            shashlikMapApi?.calculateRoute(e.x, e.y, routeCosting.value)
        }
    }

    private val gestureDetector = GestureDetector(context, gestureListener)

    private var rustBrige = RB()

    var shashlikMapApi: ShashlikMapApi? = null

    constructor(context: Context) : super(context) {
    }

    constructor(context: Context, attrs: AttributeSet) : super(context, attrs) {
    }

    constructor(context: Context, attrs: AttributeSet, defStyle: Int) : super(
        context,
        attrs,
        defStyle
    ) {
    }

    init {
        holder.addCallback(this)

        this.setZOrderMediaOverlay(true)
        holder.setFormat(PixelFormat.TRANSPARENT)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        gestureDetector.onTouchEvent(event)
        mScaleDetector.onTouchEvent(event)
        return true
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        shashlikMapApi?.resize(width.toUInt(), height.toUInt())
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        holder.let { h ->
            val ptr = rustBrige.createShashlikMapApi(
                h.surface,
                Build.FINGERPRINT.contains("generic") ||
                        Build.FINGERPRINT.contains("sdk_gphone"),
                context.filesDir.absolutePath + "/tiles.db"
            )
            shashlikMapApi = ShashlikMapApi(ptr.toPointer())
            setWillNotDraw(false)
        }
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
//        if (wgpuObj != Long.MAX_VALUE) {
//            rustBrige.dropWgpuCanvas(wgpuObj)
//            wgpuObj = Long.MAX_VALUE
//        }
    }

    override fun surfaceRedrawNeeded(holder: SurfaceHolder) {
    }


    override fun onSaveInstanceState(): Parcelable? {
        return super.onSaveInstanceState()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        shashlikMapApi?.render()
        invalidate()
    }
}