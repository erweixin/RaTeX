plugins {
    id("com.android.application") version "8.2.2" apply false
    id("org.jetbrains.kotlin.android") version "1.9.24" apply false
    id("io.github.gradle-nexus.publish-plugin") version "2.0.0"
}

// Nexus Publish 插件用根项目 group 查找 Sonatype staging profile，必须与 publication 的 groupId 一致
group = "io.github.erweixin"

// Nexus Publish 插件必须应用在根项目；实际 publication 定义在 :ratex-android (platforms/android)
nexusPublishing {
    repositories {
        sonatype {
            nexusUrl.set(uri("https://ossrh-staging-api.central.sonatype.com/service/local/"))
            snapshotRepositoryUrl.set(uri("https://central.sonatype.com/repository/maven-snapshots/"))
            username.set(project.findProperty("SONATYPE_NEXUS_USERNAME")?.toString() ?: "")
            password.set(project.findProperty("SONATYPE_NEXUS_PASSWORD")?.toString() ?: "")
        }
    }
}
