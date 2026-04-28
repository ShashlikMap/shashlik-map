package com.shashlik.kmp

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitView

@OptIn(kotlin.experimental.ExperimentalNativeApi::class)
actual val isDebugBuild: Boolean get() = Platform.isDebugBinary
@Composable
actual fun ShashlikMap(onLongTap: (x: Float, y: Float) -> Unit) {
    UIKitView(
        factory = {
            MetalUIView()
        },
        modifier = Modifier.fillMaxSize()
    )
}