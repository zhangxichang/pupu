plugins {
    `kotlin-dsl`
}

repositories {
    google()
    mavenCentral()
}

dependencies {
    implementation("com.android.tools.build:gradle:9.5.1")
    implementation("com.android.tools:sdk-common:32.5.1")

    // Only here to clear GitHub Security & Quality alerts for build-logic's
    // own build-time dependencies. These are not application dependencies.
    // Remove these pins once AGP ships patched versions.
    constraints {
        implementation("com.google.guava:guava:33.7.2-jre")
        implementation("org.bouncycastle:bcprov-jdk18on:1.85")
        implementation("org.bouncycastle:bcpkix-jdk18on:1.85")
        implementation("org.bouncycastle:bcutil-jdk18on:1.85")
        implementation("org.bitbucket.b_c:jose4j:0.9.6")
        implementation("org.jdom:jdom2:2.0.6.1")
        implementation("org.apache.commons:commons-lang3:3.18.0")
        implementation("org.apache.httpcomponents:httpclient:4.5.14")
        implementation("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21")
    }
}
