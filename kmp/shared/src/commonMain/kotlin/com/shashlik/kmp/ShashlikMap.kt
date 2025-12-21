package com.shashlik.kmp

import androidx.compose.runtime.Composable
import uniffi.ffi_run.ShashlikMapApi

@Composable
expect fun ShashlikMap(onLongTap: (x: Float, y: Float) -> Unit)

object ShashlikMapApiHolder {
    var shashlikMapApi: ShashlikMapApi? = null
}

