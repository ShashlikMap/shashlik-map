import gobley.gradle.cargo.tasks.CargoBuildTask
import groovy.json.JsonSlurper
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

tasks.withType<CargoBuildTask>().configureEach {
    val maptilerApiKey = System.getenv("MAPTILER_API_KEY")
    if (!maptilerApiKey.isNullOrEmpty()) {
        additionalEnvironment.put("MAPTILER_API_KEY", maptilerApiKey)
    }
}

uniffi {
    generateFromLibrary {
        variant = gobley.gradle.Variant.Release
    }
}

val rustlsPlatformVerifierAar = providers.exec {
    workingDir = rootDir.parentFile
    commandLine(
        "cargo", "metadata",
        "--format-version", "1",
        "--filter-platform", "aarch64-linux-android",
    )
}.standardOutput.asText.map { metadata ->
    @Suppress("UNCHECKED_CAST")
    val packages = (JsonSlurper().parseText(metadata) as Map<String, Any>)
        .getValue("packages") as List<Map<String, Any>>
    val crate = packages.firstOrNull { it["name"] == "rustls-platform-verifier-android" }
        ?: error("rustls-platform-verifier-android is not in the Cargo graph for aarch64-linux-android")
    val version = crate.getValue("version") as String
    val crateDir = File(crate.getValue("manifest_path") as String).parentFile
    File(crateDir, "maven/rustls/rustls-platform-verifier/$version/rustls-platform-verifier-$version.aar")
        .also { require(it.isFile) { "Expected the rustls-platform-verifier AAR at $it" } }
}

val unpackRustlsPlatformVerifier by tasks.registering(Copy::class) {
    description = "Extracts the Kotlin component bundled in the rustls-platform-verifier-android crate."
    from(zipTree(rustlsPlatformVerifierAar)) {
        include("classes.jar")
    }
    into(layout.buildDirectory.dir("rustlsPlatformVerifier"))
    rename("classes.jar", "rustls-platform-verifier.jar")
}

val rustlsPlatformVerifierJar = files(
    layout.buildDirectory.file("rustlsPlatformVerifier/rustls-platform-verifier.jar")
).builtBy(unpackRustlsPlatformVerifier)

kotlin {
    androidTarget {
        publishLibraryVariants("release")
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }

//    listOf(
//        iosArm64(),
//        iosSimulatorArm64()
//    ).forEach { iosTarget ->
//        iosTarget.binaries.framework {
//            baseName = "Shared"
//            isStatic = true
//        }
//    }

    sourceSets {
        androidMain.dependencies {
            implementation(libs.androidx.core.ktx)
            implementation(project.dependencies.platform(libs.androidx.compose.bom))
            implementation(libs.androidx.ui)
            implementation(libs.androidx.ui.graphics)
            implementation(libs.androidx.material3)
            implementation(libs.accompanist)
            implementation(libs.play.services.location)
            implementation(rustlsPlatformVerifierJar)
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
        consumerProguardFiles("consumer-rules.pro")
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
version = "0.2.3"

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
