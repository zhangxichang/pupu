plugins {
    id("com.android.application")
    id("launcher-icon")
    id("rust-jni")
}

android {
    namespace = "com.zhangxichang.pupu"
    ndkVersion = "30.0.16248370"
    compileSdk {
        version = release(37)
    }

    defaultConfig {
        applicationId = "com.zhangxichang.pupu"
        minSdk = 34
        targetSdk = 37
        versionCode = 1
        versionName = "1.0"
    }
}
