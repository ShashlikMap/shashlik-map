rootProject.name = "kmp"
enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

pluginManagement {
    repositories {
        google {
            mavenContent {
                includeGroupAndSubgroups("androidx")
                includeGroupAndSubgroups("com.android")
                includeGroupAndSubgroups("com.google")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google {
            mavenContent {
                includeGroupAndSubgroups("androidx")
                includeGroupAndSubgroups("com.android")
                includeGroupAndSubgroups("com.google")
            }
        }
        mavenCentral()

        // TODO so far works for rustlsPlatformVerifier, but it will likely cause issues with maven publishing
        maven {
            name = "rustlsPlatformVerifier"
            url = uri(File(System.getProperty("user.home"),
                ".cargo/registry/src").walkTopDown()
                .first { it.name == "maven" && it.path.contains("rustls-platform-verifier-android-") })
        }
    }
}

include(":shared")

include(":composeApp")
project(":composeApp").projectDir = File(rootDir, "demo/composeApp")
