package com.shashlik.demo

import androidx.compose.ui.window.ComposeUIViewController
import platform.UIKit.UIViewController

/**
 * An entry point for iosApp.
 * @see [ComposeView] of iosApp
 */
@Suppress("unused", "FunctionName")
fun MainViewController(): UIViewController {
    return ComposeUIViewController {
        App()
    }
}