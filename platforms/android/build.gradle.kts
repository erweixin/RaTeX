import java.io.File

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android") version "1.9.24"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    id("com.vanniktech.maven.publish") version "0.30.0"
}

android {
    namespace = "io.ratex"
    compileSdk = 34

    defaultConfig {
        minSdk = 21   // Android 5.0+, broad device support
        targetSdk = 34

        // Package the native .so files built by build-android.sh
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("androidx.annotation:annotation:1.7.1")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
}

// CI（release-android.yml）传入 -PlibraryVersion；本地默认与根目录 VERSION 一致
val libraryVersion = project.findProperty("libraryVersion")?.toString()?.takeIf { it.isNotBlank() }
    ?: File(rootProject.rootDir, "../../VERSION").normalize().readText().trim()

mavenPublishing {
    publishToMavenCentral(com.vanniktech.maven.publish.SonatypeHost.CENTRAL_PORTAL, automaticRelease = true)
    signAllPublications()
    coordinates("io.github.erweixin", "ratex-android", libraryVersion)

    pom {
        name.set("RaTeX Android")
        description.set("Native Android rendering of LaTeX math (Canvas + KaTeX fonts)")
        url.set("https://github.com/erweixin/RaTeX")
        licenses {
            license {
                name.set("MIT")
                url.set("https://opensource.org/licenses/MIT")
            }
        }
        scm {
            url.set("https://github.com/erweixin/RaTeX")
            connection.set("scm:git:git://github.com/erweixin/RaTeX.git")
            developerConnection.set("scm:git:ssh://git@github.com/erweixin/RaTeX.git")
        }
        developers {
            developer {
                name.set("RaTeX Contributors")
                url.set("https://github.com/erweixin/RaTeX")
            }
        }
    }
}
