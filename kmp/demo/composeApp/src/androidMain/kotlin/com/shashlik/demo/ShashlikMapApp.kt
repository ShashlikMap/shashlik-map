package com.shashlik.demo

import android.app.Application
import com.shashlik.kmp.shashlikMapInit

class ShashlikMapApp : Application() {
    override fun onCreate() {
        super.onCreate()
        shashlikMapInit()
    }
}