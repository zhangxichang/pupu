pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        mavenCentral()
    }
    plugins {
        id("com.android.application") version "9.5.1"
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "pupu"

include(":android:app")
includeBuild("android/build-logic")

// Only here to clear GitHub Security & Quality alerts for AGP's transitive
// build-time dependencies. These are not application dependencies.
// Remove these pins once AGP ships patched versions.
gradle.beforeProject {
    buildscript.configurations.maybeCreate("classpath").resolutionStrategy.force(
        "com.google.guava:guava:33.7.2-jre",
        "org.bouncycastle:bcprov-jdk18on:1.85",
        "org.bouncycastle:bcpkix-jdk18on:1.85",
        "org.bouncycastle:bcutil-jdk18on:1.85",
        "org.bitbucket.b_c:jose4j:0.9.6",
        "org.jdom:jdom2:2.0.6.1",
        "org.apache.commons:commons-lang3:3.18.0",
        "org.apache.httpcomponents:httpclient:4.5.14",
        "org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21",
    )
}
