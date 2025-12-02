import gobley.gradle.rust.targets.RustAndroidTarget
import org.gradle.kotlin.dsl.implementation
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidLibrary)
    alias(libs.plugins.gobleyCargo)
    alias(libs.plugins.gobleyUniffi)
    kotlin("plugin.atomicfu") version libs.versions.kotlin

    alias(libs.plugins.composeMultiplatform)
    alias(libs.plugins.composeCompiler)
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
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }
    
    listOf(
        iosArm64(),
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
            implementation("net.java.dev.jna:jna:5.18.1@aar")
        }
        commonMain.dependencies {
            implementation(compose.runtime)
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
    }
}
