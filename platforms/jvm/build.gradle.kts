import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import java.io.File

plugins {
    id("org.jetbrains.kotlin.jvm") version "1.9.24"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    id("com.vanniktech.maven.publish") version "0.30.0"
}

group = "io.github.erweixin"

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin.compilerOptions {
    jvmTarget = JvmTarget.JVM_17
}

sourceSets {
    main {
        resources.srcDir("native")
    }
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    implementation("net.java.dev.jna:jna:5.18.1")
}

// CI passes -PlibraryVersion; local default from root VERSION file
val libraryVersion = project.findProperty("libraryVersion")?.toString()?.takeIf { it.isNotBlank() }
    ?: File(rootProject.rootDir, "../../VERSION").normalize().readText().trim()

mavenPublishing {
    publishToMavenCentral(com.vanniktech.maven.publish.SonatypeHost.CENTRAL_PORTAL, automaticRelease = true)
    if (!project.hasProperty("skipSigning")) {
        signAllPublications()
    }
    coordinates("io.github.erweixin", "ratex-jvm", libraryVersion)

    pom {
        name.set("RaTeX JVM")
        description.set("JVM platform support for RaTeX — LaTeX math rendering via JNA + AWT Graphics2D")
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
