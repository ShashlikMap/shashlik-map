package com.shashlik.kmp

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitView

@Composable
actual fun ShashlikMap(onLongTap: (x: Float, y: Float) -> Unit) {
    UIKitView(
        factory = {
            MetalUIView()
        },
        modifier = Modifier.fillMaxSize()
    )
}