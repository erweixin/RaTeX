plugins {
    id("com.android.application") version "8.2.2" apply false
    id("org.jetbrains.kotlin.android") version "1.9.24" apply false
    id("maven-publish")
    id("signing")
    id("io.github.gradle-nexus.publish-plugin") version "2.0.0"
}

// Maven Central：根项目发布 ratex-android 并自动 Close & Release（Nexus Publish 插件要求应用在根项目）
nexusPublishing {
    repositories {
        sonatype {
            // 使用新版 Central API，避免 s01.oss.sonatype.com 在部分环境（如 GitHub Actions）DNS 解析失败
            nexusUrl.set(uri("https://ossrh-staging-api.central.sonatype.com/service/local/"))
            snapshotRepositoryUrl.set(uri("https://central.sonatype.com/repository/maven-snapshots/"))
            username.set(project.findProperty("SONATYPE_NEXUS_USERNAME")?.toString() ?: "")
            password.set(project.findProperty("SONATYPE_NEXUS_PASSWORD")?.toString() ?: "")
        }
    }
}

gradle.projectsEvaluated {
    val r = project(":ratex-android")
    publishing {
        publications {
            create<MavenPublication>("release") {
                groupId = "io.github.erweixin"
                artifactId = "ratex-android"
                version = r.version.toString()
                from(r.components["release"])
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
    }
    when {
        project.hasProperty("signing.gnupg.keyName") -> {
            signing {
                useGpgCmd()
                sign(publishing.publications["release"])
            }
        }
        project.hasProperty("signing.keyId") -> {
            signing {
                sign(publishing.publications["release"])
            }
        }
    }
}
