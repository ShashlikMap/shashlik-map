import java.io.FileInputStream
import java.util.Properties

rootProject.name = "kmp"
enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

val properties = Properties()
properties.load(FileInputStream(file("local.properties")))

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
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/ShashlikMap/*")
            credentials {
                username = properties.getProperty("GithubUser")
                password = properties.getProperty("GithubPAT")
            }
        }
    }
}

include(":shared")
//include(":demo")

include(":composeApp")
project(":composeApp").projectDir = File(rootDir, "demo/composeApp")
