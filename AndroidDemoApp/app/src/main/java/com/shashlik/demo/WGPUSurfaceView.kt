package com.shashlik.demo

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.PixelFormat
import android.os.Build
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import com.sun.jna.Pointer
import uniffi.ffi_run.ShashlikMapApi
import uniffi.ffi_run.UniffiWithHandle

@SuppressLint("ClickableViewAccessibility")
class WGPUSurfaceView : SurfaceView, SurfaceHolder.Callback2 {
    private var rustBrige = RB()

    private var shashlikMapApi: ShashlikMapApi? = null

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

        setOnTouchListener { _, event ->
            when(event.action) {
                MotionEvent.ACTION_DOWN -> {
                    shashlikMapApi?.tempExternalInput(true)
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                    shashlikMapApi?.tempExternalInput(false)
                }
            }
            true
        }
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        holder.let { h ->
            val ptr = rustBrige.createShashlikMapApi(h.surface,
                Build.FINGERPRINT.contains("generic") ||
                        Build.FINGERPRINT.contains("sdk_gphone"),
                context.filesDir.absolutePath+"/tiles.db"
            )
            shashlikMapApi = ShashlikMapApi(UniffiWithHandle, ptr)
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

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        shashlikMapApi?.render()
        invalidate()
    }
}