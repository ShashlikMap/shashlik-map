package com.shashlik.kmp

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.SurfaceTexture
import android.os.Build
import android.util.AttributeSet
import android.view.Surface
import android.view.TextureView
import timber.log.Timber
import uniffi.ffi_run.ShashlikMapApi
import uniffi.ffi_run.toPointer


@SuppressLint("ClickableViewAccessibility")
class WGPUTextureView : TextureView {
    companion object {
        @JvmStatic external fun initRustlsPlatformVerifier(context: Context)
    }

    init {
        System.loadLibrary("ffi_run")
    }

    external fun createShashlikMapApi(surface: Surface, isEmulator: Boolean, tilesDb: String, dpiScale: Float): Long

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
        // fyi, ideally it should be called during App launch only once
        initRustlsPlatformVerifier(context = context.applicationContext)
        Timber.d("WGPUTextureView created")

        surfaceTextureListener = object : SurfaceTextureListener {
            override fun onSurfaceTextureAvailable(
                st: SurfaceTexture,
                width: Int,
                height: Int
            ) {
                val surface = Surface(st)

                val ptr = createShashlikMapApi(
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
}