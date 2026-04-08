import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("org.jetbrains.kotlin.jvm") version "1.9.24"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    application
}

group = "io.ratex.demo"
version = "1.0.0"

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin.compilerOptions {
    jvmTarget = JvmTarget.JVM_17
}

application {
    mainClass.set("io.ratex.demo.MainKt")

    // Set JNA library path to find the native library built by build-jvm.sh
    val nativeDir = file("../../platforms/jvm/native").absolutePath
    val releaseDir = file("../../target/release").absolutePath
    applicationDefaultJvmArgs = listOf(
        "-Djna.library.path=$nativeDir:$releaseDir"
    )
}

dependencies {
    implementation(project(":ratex-jvm"))
}
