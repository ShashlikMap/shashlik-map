package com.shashlik.demo2.demo2

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.UIKitViewController
import androidx.compose.ui.window.ComposeUIViewController
import platform.UIKit.UIViewController


fun MainViewController(createUIViewController: () -> UIViewController) = ComposeUIViewController {
    UIKitViewController(
        factory = createUIViewController,
        modifier = Modifier.fillMaxSize(),
    )
    App()
}