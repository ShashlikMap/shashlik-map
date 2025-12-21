package com.shashlik.demo2.demo2

import androidx.compose.ui.window.ComposeUIViewController
import com.shashlik.kmp.ShashlikMapApiHolder
import com.shashlik.kmp.ShashlikMapUIViewProvider
import platform.UIKit.UIViewController
import uniffi.ffi_run.ShashlikMapApi

// FIXME Should be in Shared module
fun createShashlikMapApiForIos(view: ULong, metalLayer: ULong): ShashlikMapApi {
    val api = uniffi.ffi_run.createShashlikMapApiForIos(view, metalLayer, 90, "")
    ShashlikMapApiHolder.shashlikMapApi = api
    return api
}

fun MainViewController(createUIViewController: () -> UIViewController): UIViewController {
    ShashlikMapUIViewProvider.createUIViewController = createUIViewController
    return ComposeUIViewController {
        App()
    }
}