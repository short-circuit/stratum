val keystorePath = System.getenv("ANDROID_KEYSTORE_PATH")
if (keystorePath != null) {
    android {
        signingConfigs {
            create("release") {
                storeFile = file(keystorePath)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    afterEvaluate {
        android.buildTypes.getByName("release").signingConfig = android.signingConfigs.getByName("release")
    }
}
