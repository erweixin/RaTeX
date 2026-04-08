pluginManagement {
    repositories {
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.PREFER_SETTINGS)
    repositories {
        mavenCentral()
    }
}

rootProject.name = "RaTeXJvmDemo"
include(":ratex-jvm")
project(":ratex-jvm").projectDir = file("../../platforms/jvm")
