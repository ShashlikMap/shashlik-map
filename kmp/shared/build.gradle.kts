import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidLibrary)
    alias(libs.plugins.gobleyCargo)
    alias(libs.plugins.gobleyUniffi)
    kotlin("plugin.atomicfu") version libs.versions.kotlin

    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
    alias(libs.plugins.vanniktech.mavenPublish)
}

cargo {
    // The Cargo package is located in a `rust` subdirectory.
    packageDirectory = layout.projectDirectory.dir("../../ffi-run")
}

uniffi {
    generateFromLibrary {
        variant = gobley.gradle.Variant.Release
    }
}

kotlin {
    androidTarget {
        publishLibraryVariants("release")
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }
    
    listOf(
//        iosArm64(), // debug builds do not work due to some linked error
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "Shared"
            isStatic = true
        }
    }
    
    sourceSets {
        androidMain.dependencies {
            implementation(libs.androidx.core.ktx)
            implementation(project.dependencies.platform(libs.androidx.compose.bom))
            implementation(libs.androidx.ui)
            implementation(libs.androidx.ui.graphics)
            implementation(libs.androidx.material3)
            implementation(libs.accompanist)
            implementation("net.java.dev.jna:jna:5.18.1@aar")
            implementation("com.jakewharton.timber:timber:5.0.1")
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.ui)
        }
        commonTest.dependencies {
            implementation(libs.kotlin.test)
        }
    }
}

android {
    namespace = "com.shashlik.kmp.shared"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()
        ndk {
            //noinspection ChromeOsAbiSupport
            abiFilters += listOf("arm64-v8a")
        }
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
}

group = "io.github.shashlikmap"
version = "0.2.1"

mavenPublishing {
    publishToMavenCentral()

    signAllPublications()

    coordinates(group.toString(), "mapshared", version.toString())

    pom {
        name = "ShashlikMapSDK"
        description = "WIP Map SDK powered by KMP and Rust WGPU"
        inceptionYear = "2025"
        url = "https://github.com/ShashlikMap/shashlik-map"
        licenses {
            license {
                name = "The Apache License, Version 2.0"
                url = "https://www.apache.org/licenses/LICENSE-2.0.txt"
                distribution = "https://www.apache.org/licenses/LICENSE-2.0.txt"
            }
        }
        developers {
            developer {
                id = "ShashlikMap"
                name = "ShashlikMap"
                url = "https://github.com/ShashlikMap"
                email = "olenyov.kirill@me.com"
                organization = "ShashlikMap"
                organizationUrl = "https://github.com/ShashlikMap"
            }
        }
        scm {
            url = "https://github.com/ShashlikMap/shashlik-map"
            connection = "scm:git:git://github.com/ShashlikMap/shashlik-map.git"
            developerConnection = "scm:git:ssh://git@github.com/ShashlikMap/shashlik-map.git"
        }
    }
}
