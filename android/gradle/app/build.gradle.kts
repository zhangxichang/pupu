plugins {
    id("com.android.application")
    id("launcher-icon")
    id("rust-jni")
}

fun optionalEnv(name: String): String? =
    providers.environmentVariable(name).orNull?.takeIf { it.isNotBlank() }

val releaseKeystorePath = optionalEnv("ANDROID_KEYSTORE_PATH")
val releaseKeystorePassword = optionalEnv("ANDROID_KEYSTORE_PASSWORD")
val releaseKeyAlias = optionalEnv("ANDROID_KEYSTORE_KEYALIAS")
val releaseKeyPassword = optionalEnv("ANDROID_KEYSTORE_KEY_PASSWORD") ?: releaseKeystorePassword

val appVersionName = providers.gradleProperty("versionName").orElse("0.0.0")
val appVersionCode = providers.gradleProperty("versionCode").orElse("1").map(String::toInt)

android {
    namespace = "com.zhangxichang.pupu"
    ndkVersion = "30.0.16248370"
    compileSdk {
        version = release(34)
    }

    defaultConfig {
        applicationId = "com.zhangxichang.pupu"
        minSdk = 34
        targetSdk = 34
        versionCode = appVersionCode.get()
        versionName = appVersionName.get()
    }

    signingConfigs {
        if (releaseKeystorePath != null) {
            require(releaseKeystorePassword != null) {
                "ANDROID_KEYSTORE_PASSWORD 未设置，无法签名 release 构建"
            }
            require(releaseKeyAlias != null) {
                "ANDROID_KEYSTORE_KEYALIAS 未设置，无法签名 release 构建"
            }
            create("release") {
                storeFile = file(releaseKeystorePath)
                storeType = "PKCS12"
                storePassword = releaseKeystorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("release")
        }
    }
}
