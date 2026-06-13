package com.shashlik.kmp

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitInteropInteractionMode
import androidx.compose.ui.viewinterop.UIKitInteropProperties
import androidx.compose.ui.viewinterop.UIKitView

@OptIn(kotlin.experimental.ExperimentalNativeApi::class)
actual val isDebugBuild: Boolean get() = Platform.isDebugBinary
@Composable
actual fun ShashlikMap() {
    UIKitView(
        factory = {
            MetalUIView()
        },
        modifier = Modifier.fillMaxSize(),
        // disable interactionMode to give a full control to Compose
        // otherwise some gesture may not work correctly
        properties = UIKitInteropProperties(
            interactionMode = null
        )
    )
}