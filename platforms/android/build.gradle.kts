plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android") version "1.9.24"
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.24"
    id("maven-publish")
    id("signing")
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
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
}

// Maven publish — AAR (with fonts in assets) to Maven Local or remote
// Override in CI with -PlibraryVersion=x.y.z (e.g. from git tag)
val libraryVersion = project.findProperty("libraryVersion")?.toString() ?: "0.0.3"
group = "io.github.erweixin"
version = libraryVersion

afterEvaluate {
    tasks.matching { it.name.endsWith("SourcesJar") }.configureEach {
        (this as org.gradle.api.tasks.bundling.Jar).duplicatesStrategy = org.gradle.api.file.DuplicatesStrategy.EXCLUDE
    }
    publishing {
        publications {
            create<MavenPublication>("release") {
                groupId = "io.github.erweixin"
                artifactId = "ratex-android"
                version = libraryVersion
                from(components["release"])
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
        }
        repositories {
            mavenLocal()
            // GitHub Packages: MAVEN_REPO_URL / MAVEN_USER / MAVEN_PASSWORD
            maven {
                name = "remote"
                url = uri(project.findProperty("MAVEN_REPO_URL")?.toString() ?: "https://maven.pkg.github.com/erweixin/RaTeX")
                credentials {
                    username = project.findProperty("MAVEN_USER")?.toString() ?: ""
                    password = project.findProperty("MAVEN_PASSWORD")?.toString() ?: ""
                }
            }
            // Maven Central 由根项目 Nexus Publish 插件统一发布，此处仅保留 GitHub/本地
        }
    }
    // Maven Central 要求 GPG 签名；未配置 signing 时跳过，便于本地/CI 仅发 GitHub 等
    if (project.hasProperty("signing.keyId")) {
        signing {
            sign(publishing.publications["release"])
        }
    }
}
