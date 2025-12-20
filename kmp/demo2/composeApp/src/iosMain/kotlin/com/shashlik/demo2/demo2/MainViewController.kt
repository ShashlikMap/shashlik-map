package com.shashlik.demo2.demo2

import androidx.compose.ui.window.ComposeUIViewController
import com.shashlik.kmp.ShashlikMapUIViewProvider
import platform.UIKit.UIViewController

fun MainViewController(createUIViewController: () -> UIViewController): UIViewController {
    ShashlikMapUIViewProvider.createUIViewController = createUIViewController
    return ComposeUIViewController {
        App()
    }
}