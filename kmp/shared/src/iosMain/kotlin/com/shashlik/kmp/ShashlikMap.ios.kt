package com.shashlik.kmp

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitViewController
import platform.UIKit.UIViewController

object ShashlikMapUIViewProvider {
    lateinit var createUIViewController: () -> UIViewController
}

@Composable
actual fun ShashlikMap(onLongTap: (x: Float, y: Float) -> Unit) {
    // TODO Pass to iOS
    UIKitViewController(
        factory = ShashlikMapUIViewProvider.createUIViewController,
        modifier = Modifier.fillMaxSize(),
    )
}