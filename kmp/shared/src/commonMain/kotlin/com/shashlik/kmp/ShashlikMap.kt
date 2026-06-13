package com.shashlik.kmp

import androidx.compose.runtime.Composable
import uniffi.ffi_run.ShashlikMapApi

expect val isDebugBuild: Boolean

@Composable
expect fun ShashlikMap()

object ShashlikMapApiHolder {
    var shashlikMapApi: ShashlikMapApi? = null
}

