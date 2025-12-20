package com.shashlik.demo2.demo2

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform