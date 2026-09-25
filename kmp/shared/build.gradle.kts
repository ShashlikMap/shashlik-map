import gobley.gradle.cargo.tasks.CargoBuildTask
import groovy.json.JsonSlurper
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.plugin.mpp.NativeBuildType

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

@Suppress("UNCHECKED_CAST")
val rustlsPlatformVerifierVersion = rootProject.extra["rustlsPlatformVerifierVersion"] as Provider<String>

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

kotlin {
    // Selects the JDK that runs kotlinc/javac, independently of the JDK used to
    // launch Gradle. Without this, AGP's JdkImageTransform runs jlink from
    // whatever JDK you happen to have (it fails on GraalVM 21). Published
    // bytecode is unaffected: jvmTarget and compileOptions below stay at 11.
    jvmToolchain(17)

    androidTarget {
        publishLibraryVariants("release")
        compilerOptions {
            jvmTarget.set(JvmTarget.JVM_11)
        }
    }

    // TODO iOS targe temporary disabled.
    //  It takes a lot of time to build locally and on CI and produces huge binaries(MVN complains)
    //  Need to figure out the reason later.
//    listOf(
//        iosArm64(),
//        iosSimulatorArm64()
//    ).forEach { iosTarget ->
//        iosTarget.binaries.framework(listOf(NativeBuildType.RELEASE)) {
//            baseName = "Shared"
//            isStatic = true
//        }
//    }

    sourceSets {
        all {
            languageSettings.optIn("com.shashlik.kmp.InternalShashlikMapApi")
        }
        androidMain.dependencies {
            implementation(libs.androidx.core.ktx)
            implementation(project.dependencies.platform(libs.androidx.compose.bom))
            implementation(libs.androidx.ui)
            implementation(libs.androidx.ui.graphics)
            implementation(libs.androidx.material3)
            implementation(libs.accompanist)
            implementation(libs.play.services.location)
            implementation("org.rustls:rustls-platform-verifier:${rustlsPlatformVerifierVersion.get()}")
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
version = "0.3.23"

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

// ---------------------------------------------------------------------------
// Agent-facing docs: build-derived facts.
//
// Everything below is read out of the build itself, so it cannot drift from
// what is actually published. Prose docs should include this file rather than
// restate any of it by hand.
// ---------------------------------------------------------------------------
val agentFacts by tasks.registering {
    group = "documentation"
    description = "Regenerates agent/FACTS.md from the build configuration."

    val outFile = layout.projectDirectory.file("agent/FACTS.md").asFile
    val manifest = layout.projectDirectory.file("src/androidMain/AndroidManifest.xml").asFile

    // Captured at configuration time so the task body stays configuration-cache safe.
    val coordinates = "${project.group}:mapshared:${project.version}"
    val minSdkValue = libs.versions.android.minSdk.get()
    val compileSdkValue = libs.versions.android.compileSdk.get()
    val abis = android.defaultConfig.ndk.abiFilters.sorted()
    val targetNames = kotlin.targets.map { it.name }.filterNot { it == "metadata" }.sorted()
    val jvmTargetValue = JavaVersion.VERSION_11.toString()

    inputs.file(manifest)
    inputs.property("coordinates", coordinates)
    inputs.property("minSdk", minSdkValue)
    inputs.property("compileSdk", compileSdkValue)
    inputs.property("jvmTarget", jvmTargetValue)
    inputs.property("abis", abis)
    inputs.property("targets", targetNames)
    outputs.file(outFile)

    doLast {
        val permissions = Regex("""android:name="android\.permission\.([A-Z_]+)"""")
            .findAll(manifest.readText())
            .map { it.groupValues[1] }
            .distinct()
            .sorted()
            .toList()

        val hasIos = targetNames.any { it.startsWith("ios") }
        val platforms = if (hasIos) {
            "Android, iOS"
        } else {
            "**Android only** — iOS targets are not built or published"
        }

        outFile.parentFile.mkdirs()
        outFile.writeText(
            buildString {
                appendLine("<!-- GENERATED by `./gradlew :shared:agentFacts` — do not edit by hand. -->")
                appendLine()
                appendLine("# mapshared build facts")
                appendLine()
                appendLine("| | |")
                appendLine("|---|---|")
                appendLine("| Coordinates | `$coordinates` |")
                appendLine("| Repository | `mavenCentral()` |")
                appendLine("| Platforms | $platforms |")
                appendLine("| Kotlin targets | ${targetNames.joinToString(", ") { "`$it`" }} |")
                appendLine("| Android minSdk | $minSdkValue |")
                appendLine("| Android compileSdk | $compileSdkValue |")
                appendLine("| JVM target | $jvmTargetValue |")
                appendLine("| Native ABIs | ${abis.joinToString(", ") { "`$it`" }} |")
                appendLine(
                    "| Manifest permissions | ${permissions.joinToString(", ") { "`$it`" }} |"
                )
                appendLine()
                appendLine("Permissions above are merged into the consuming app automatically;")
                appendLine("they do not need to be declared again.")
                appendLine()
                appendLine("Public API signatures: [`api/shared.api`](../api/shared.api),")
                appendLine("regenerated by `./gradlew apiDump`.")
            }
        )
        logger.lifecycle("Wrote $outFile")
    }
}

/**
 * Substitutes the machine-generated regions of README_API.md.
 *
 * Only text between the BEGIN/END GENERATED markers is touched. Prose outside
 * the markers is hand-written and is never rewritten here, so this task can run
 * on every release without risking the curated sections.
 */
val agentDocs by tasks.registering {
    group = "documentation"
    description = "Refreshes the generated regions of README_API.md."

    dependsOn(agentFacts, tasks.named("apiDump"))

    val readme = layout.projectDirectory.file("README_API.md").asFile
    val factsFile = layout.projectDirectory.file("agent/FACTS.md").asFile
    val apiFile = layout.projectDirectory.file("api/shared.api").asFile
    val versionValue = "${project.version}"
    // The Gradle root is `kmp/`; llms.txt belongs at the repository root.
    val llmsTxt = File(rootDir.parentFile, "llms.txt")
    val rootReadme = File(rootDir.parentFile, "README.md")
    val groupValue = "${project.group}"
    val minSdkForDocs = libs.versions.android.minSdk.get()
    val abisForDocs = android.defaultConfig.ndk.abiFilters.sorted().joinToString(", ")

    inputs.files(factsFile, apiFile)
    inputs.property("version", versionValue)
    inputs.property("group", groupValue)
    inputs.property("minSdk", minSdkForDocs)
    inputs.property("abis", abisForDocs)
    // All three are rewritten by this task. Declaring only README_API.md let a
    // deleted or edited llms.txt / root README survive an UP-TO-DATE run, which
    // would then pass the publish gate while stale.
    outputs.files(readme, llmsTxt, rootReadme)

    doLast {
        // --- collect the real public surface from the BCV dump ---
        val topLevelFunctions = sortedSetOf<String>()
        val extensionProperties = sortedSetOf<String>()
        val types = sortedSetOf<String>()
        var inKtFacade = false

        apiFile.readLines().forEach { raw ->
            val line = raw.trim()
            when {
                line.startsWith("public") && line.contains(" class ") -> {
                    val fqcn = line.substringAfter(" class ")
                        .substringBefore(" ")
                        .removeSuffix("{")
                        .trim()
                        .replace('/', '.')
                    val simple = fqcn.substringAfterLast('.')
                    inKtFacade = simple.endsWith("Kt")
                    if (!inKtFacade && !simple.contains('$')) {
                        types += simple
                    }
                }
                inKtFacade && line.contains(" fun ") -> {
                    val name = line.substringAfter(" fun ")
                        .substringBefore(" ")
                        .substringBefore("(")
                        .substringBefore("-")   // drop value-class mangling
                        .trim()
                    val accessor = Regex("^(get|set)([A-Z].*)").find(name)
                    when {
                        name.endsWith("\$default") || name.startsWith("access\$") -> Unit
                        // A Kotlin extension property appears in the JVM dump as a
                        // getX/setX accessor. Listing it as a function invites callers
                        // to write `getWidth(shape)` instead of `shape.width`.
                        accessor != null -> extensionProperties +=
                            accessor.groupValues[2].replaceFirstChar { it.lowercase() }
                        else -> topLevelFunctions += name
                    }
                }
            }
        }

        // --- build the replacement blocks ---
        val factsTable = factsFile.readLines()
            .dropWhile { !it.startsWith("|") }
            .takeWhile { it.startsWith("|") }
            .joinToString("\n")

        val inventory = buildString {
            appendLine("Derived from `api/shared.api`. A name here with no section in this")
            appendLine("document means the document is incomplete. The reverse does **not**")
            appendLine("hold: this is not an exhaustive list of supported API (see the note")
            appendLine("at the end), so never delete a section merely because it is absent here.")
            appendLine()
            appendLine("**Top-level functions**")
            topLevelFunctions.forEach { appendLine("- `$it`") }
            if (extensionProperties.isNotEmpty()) {
                appendLine()
                appendLine("**Extension properties** — call these as properties, not functions.")
                appendLine("They appear in `shared.api` as JVM `getX`/`setX` accessors.")
                extensionProperties.forEach { appendLine("- `$it`") }
            }
            appendLine()
            appendLine("**Types**")
            types.forEach { appendLine("- `$it`") }
            appendLine()
            appendLine("Not listed: `uniffi.ffi_run` types (`Point`, `Color`, `ShapeType`,")
            appendLine("`ShashlikMapApi`) are excluded from `shared.api`, so their absence here")
            appendLine("does **not** mean they are unsupported. See Known limitations.")
        }.trim()

        // --- substitute ---
        var replaced = 0

        fun substitute(source: String, file: File, name: String, body: String): String {
            val begin = "<!-- BEGIN GENERATED: $name -->"
            val end = "<!-- END GENERATED: $name -->"
            val pattern = Regex(
                Regex.escape(begin) + ".*?" + Regex.escape(end),
                RegexOption.DOT_MATCHES_ALL
            )
            require(pattern.containsMatchIn(source)) {
                "agentDocs: generated marker '$name' is missing from ${file.name}. " +
                    "Restore the '$begin' / '$end' pair. Warning and continuing here " +
                    "would leave a stale region that the publish gate cannot detect, " +
                    "because an unchanged file produces no diff."
            }
            replaced++
            return pattern.replace(source) { "$begin\n$body\n$end" }
        }

        var text = readme.readText()

        text = substitute(text, readme, "version", "This document describes **mapshared $versionValue**.")
        text = substitute(text, readme, "facts", factsTable)
        text = substitute(text, readme, "inventory", inventory)

        readme.writeText(text)
        logger.lifecycle("agentDocs: refreshed $replaced generated region(s) in ${readme.name}")

        // Entry point for foreign agents. Links raw URLs: GitHub HTML pages are
        // mostly navigation noise once a model is reading them.
        val raw = "https://raw.githubusercontent.com/ShashlikMap/shashlik-map/main"
        llmsTxt.writeText(
            """
            # Shashlik Map SDK

            > Android map SDK powered by a Rust/WGPU engine, with a Compose-first API:
            > a `ShashlikMap` composable, overlays declared in its content slot.
            > Published as `io.github.shashlikmap:mapshared` on Maven Central.

            This describes **mapshared $versionValue**. If the version you resolved
            differs, treat these documents as unreliable. If the compiler disagrees
            with them, the compiler is right: stop and tell the user rather than
            working around it. Do not use any API that is absent from the reference
            below, even if it seems like it should exist.

            Important:
            - `Point(x, y)` means x = longitude, y = latitude. `LocationState` uses
              named `latitude` / `longitude`. Double-check every coordinate.
            - Android only, `arm64-v8a` only, minSdk 26. There is no iOS artifact.
            - Map tiles cover Japan and the SF Bay Area only. Test with coordinates there.
            - **From 0.3.21 you must add the rustls Maven repository** to
              `settings.gradle.kts`, or the build cannot resolve
              `org.rustls:rustls-platform-verifier`. It is on none of the usual
              repositories. See the Setup section of the API reference.

            ## Docs
            - [API reference]($raw/kmp/shared/README_API.md): public API of the `:shared` module, with examples
            - [Build facts]($raw/kmp/shared/agent/FACTS.md): coordinates, minSdk, ABIs, permissions
            - [Public API surface]($raw/kmp/shared/api/shared.api): exact signatures, machine-generated
            - [Demo app](https://github.com/ShashlikMap/shashlik-map/tree/main/kmp/demo): runnable reference integration
            """.trimIndent() + "\n"
        )
        logger.lifecycle("agentDocs: wrote ${llmsTxt.path}")

        // Root README is public-facing and hand-edited, so it carries no
        // BEGIN/END markers. Instead we rewrite the few machine-owned values in
        // place. Each pattern must match exactly once: if someone reformats the
        // section, this fails loudly rather than silently leaving a stale version.
        var readmeText = rootReadme.readText()

        fun rewrite(pattern: Regex, replacement: String) {
            val hits = pattern.findAll(readmeText).count()
            require(hits == 1) {
                "agentDocs: expected exactly 1 match for /${pattern.pattern}/ in " +
                    "${rootReadme.name}, found $hits. The integration section was " +
                    "edited in a way this task no longer recognises - fix the " +
                    "pattern in shared/build.gradle.kts or restore the wording."
            }
            readmeText = pattern.replace(readmeText, replacement)
        }

        rewrite(
            Regex("""shashlikMap = "[^"]+""""),
            "shashlikMap = \"$versionValue\""
        )
        rewrite(
            Regex("""module = "[^"]*:mapshared""""),
            "module = \"$groupValue:mapshared\""
        )
        rewrite(
            Regex("""Android minSdk \d+"""),
            "Android minSdk $minSdkForDocs"
        )
        rewrite(
            Regex("""an [a-z0-9-]+ device or\n?\s*emulator"""),
            "an $abisForDocs device or\nemulator"
        )

        rootReadme.writeText(readmeText)
        logger.lifecycle("agentDocs: refreshed 4 value(s) in ${rootReadme.name}")
    }
}
