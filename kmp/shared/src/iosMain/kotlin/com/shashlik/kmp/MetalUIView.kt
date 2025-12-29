package com.shashlik.kmp

import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.ObjCAction
import kotlinx.cinterop.ObjCClass
import kotlinx.cinterop.cValue
import kotlinx.cinterop.objcPtr
import kotlinx.cinterop.useContents
import platform.CoreGraphics.CGRectMake
import platform.Foundation.NSDefaultRunLoopMode
import platform.Foundation.NSRunLoop
import platform.Foundation.NSSelectorFromString
import platform.QuartzCore.CADisplayLink
import platform.QuartzCore.CAFrameRateRange
import platform.QuartzCore.CAMetalLayer
import platform.UIKit.UIColor
import platform.UIKit.UIScreen
import platform.UIKit.UIView
import platform.UIKit.UIViewMeta
import platform.darwin.NSObject
import uniffi.ffi_run.ShashlikMapApi
import uniffi.ffi_run.createShashlikMapApiForIos

/**
 * Kotlin/Native implementation of UIView with CAMetalLayer to communicate with [WgpuAppApi]
 * This is much more convenient than Swift implementation
 */
@OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)
internal class MetalUIView : UIView(CGRectMake(0.0, 0.0, 0.0, 0.0)) {
    private var displayLink: CADisplayLink? = null
    private var shashlikMapApi: ShashlikMapApi? = null

    companion object Companion : UIViewMeta() {
        override fun layerClass(): ObjCClass {
            return CAMetalLayer
        }
    }

    init {
        contentScaleFactor = UIScreen.mainScreen.scale()
        backgroundColor = UIColor.redColor
    }

    @ObjCAction
    override fun didMoveToWindow() {
        super.didMoveToWindow()
        if (window == null) {
            displayLink?.invalidate()
            displayLink = null
        }
    }

    @ObjCAction
    override fun layoutSubviews() {
        super.layoutSubviews()
        val boundsWidth = bounds.useContents { size.width }
        val boundsHeight = bounds.useContents { size.height }
        if (boundsWidth > 0 && boundsWidth > 0) {
            initializeApiIfNeeded()
            shashlikMapApi?.resize(boundsWidth.toUInt(), boundsHeight.toUInt())
        }
    }

    private fun initializeApiIfNeeded() {
        if (shashlikMapApi != null) return

        val opaquePtrThis = this.objcPtr().toLong()
        val opaquePtrLayer = this.layer.objcPtr().toLong()

        // @see ffi-run/src/platform/ios.rs
        val api = createShashlikMapApiForIos(
            opaquePtrThis.toULong(), opaquePtrLayer.toULong(), 90, ""
        )
        shashlikMapApi = api
        ShashlikMapApiHolder.shashlikMapApi = api

        startRendering()
    }

    private fun startRendering() {
        displayLink = CADisplayLink.displayLinkWithTarget(
            target = DisplayLinkProxy {
                shashlikMapApi?.render()
            }, selector = NSSelectorFromString(DisplayLinkProxy::handleDisplayLinkTick.name)
        )

        displayLink?.preferredFrameRateRange = cValue<CAFrameRateRange> {
            minimum = 30F
            preferred = 60F
            maximum = 60F
        }
        displayLink?.addToRunLoop(NSRunLoop.mainRunLoop, NSDefaultRunLoopMode)
    }
}

@OptIn(BetaInteropApi::class)
private class DisplayLinkProxy(
    private val callback: () -> Unit
) : NSObject() {
    @ObjCAction
    fun handleDisplayLinkTick() {
        callback()
    }
}