package com.shashlik.kmp

import android.view.Surface

class RB {

    init {
        System.loadLibrary("ffi_run")
    }

    external fun createShashlikMapApi(surface: Surface, isEmulator: Boolean, tilesDb: String, dpiScale: Float): Long
}