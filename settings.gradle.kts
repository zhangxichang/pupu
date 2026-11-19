pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
}

rootProject.name = "pupu"

include(":android:app")
includeBuild("android/build-logic")
