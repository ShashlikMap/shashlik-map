package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.SurfaceTexture
import android.os.Build
import android.util.AttributeSet
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.Surface
import android.view.TextureView
import timber.log.Timber
import uniffi.ffi_run.ShashlikMapApi
import uniffi.ffi_run.toPointer


@SuppressLint("ClickableViewAccessibility")
class WGPUTextureView : TextureView {

    var onLongTap: (x: Float, y: Float) -> Unit = { _, _ -> }

    private val scaleListener = object : ScaleGestureDetector.SimpleOnScaleGestureListener() {

        override fun onScale(detector: ScaleGestureDetector): Boolean {
            ShashlikMapApiHolder.shashlikMapApi?.zoomDelta(
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
                ShashlikMapApiHolder.shashlikMapApi?.pitchDelta(-distanceY / 10.0f)
            } else {
                ShashlikMapApiHolder.shashlikMapApi?.panDelta(distanceX / 15.0f, distanceY / 15.0f)
            }

            return super.onScroll(e1, e2, distanceX, distanceY)
        }

        override fun onLongPress(e: MotionEvent) {
            super.onLongPress(e)
            onLongTap(e.x, e.y)
        }
    }

    private val gestureDetector = GestureDetector(context, gestureListener)

    private var rustBrige = RustBridge()

    constructor(context: Context) : super(context) {
    }

    constructor(context: Context, attrs: AttributeSet) : super(context, attrs) {
    }

    constructor(context: Context, attrs: AttributeSet, defStyle: Int) : super(
        context,
        attrs,
        defStyle
    )

    init {
        Timber.d("WGPUTextureView created")

        surfaceTextureListener = object : SurfaceTextureListener {
            override fun onSurfaceTextureAvailable(
                st: SurfaceTexture,
                width: Int,
                height: Int
            ) {
                val surface = Surface(st)
                val ptr = rustBrige.createShashlikMapApi(
                    surface,
                    Build.FINGERPRINT.contains("generic") ||
                            Build.FINGERPRINT.contains("sdk_gphone"),
                    context.filesDir.absolutePath + "/tiles.db",
                    context.resources.displayMetrics.density / 2.0f
                )
                Timber.d("surfaceCreated = $ptr, surface = $surface")

                ShashlikMapApiHolder.shashlikMapApi = ShashlikMapApi(ptr.toPointer()).apply {
                    resize(width.toUInt(), height.toUInt())
                    render()
                }
            }

            override fun onSurfaceTextureSizeChanged(
                p0: SurfaceTexture,
                width: Int,
                height: Int
            ) {
                Timber.d("onSurfaceTextureSizeChanged $width, $height")
                ShashlikMapApiHolder.shashlikMapApi?.resize(width.toUInt(), height.toUInt())
            }

            override fun onSurfaceTextureDestroyed(p0: SurfaceTexture): Boolean {
                Timber.d("onSurfaceTextureDestroyed")
                return true
            }

            override fun onSurfaceTextureUpdated(p0: SurfaceTexture) {
                ShashlikMapApiHolder.shashlikMapApi?.render()
            }
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        gestureDetector.onTouchEvent(event)
        mScaleDetector.onTouchEvent(event)
        return true
    }
}