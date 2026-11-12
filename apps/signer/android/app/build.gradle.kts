import java.io.File
import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// Release signing is optional so local builds keep working. The release key is resolved from, in
// order of precedence:
//   1. android/key.properties, written by CI from the Android signing secrets;
//   2. ANDROID_KEYSTORE / ANDROID_KEYSTORE_PASSWORD / ANDROID_KEY_ALIAS / ANDROID_KEY_PASSWORD
//      (see ~/android-release/env.sh);
//   3. the shared ~/android-release/signing.properties used by local projects.
// Without any of them the release build falls back to the debug key (as before).
val keystoreProperties = Properties()
val keystorePropertiesFile = rootProject.file("key.properties")
val sharedSigningFile = File(System.getProperty("user.home"), "android-release/signing.properties")
val envKeystore = System.getenv("ANDROID_KEYSTORE")
val hasReleaseKey =
    keystorePropertiesFile.exists() || !envKeystore.isNullOrBlank() || sharedSigningFile.exists()
if (keystorePropertiesFile.exists()) {
    keystoreProperties.load(FileInputStream(keystorePropertiesFile))
} else if (!envKeystore.isNullOrBlank()) {
    keystoreProperties["storeFile"] = envKeystore
    keystoreProperties["storePassword"] = System.getenv("ANDROID_KEYSTORE_PASSWORD") ?: ""
    keystoreProperties["keyAlias"] = System.getenv("ANDROID_KEY_ALIAS") ?: ""
    keystoreProperties["keyPassword"] = System.getenv("ANDROID_KEY_PASSWORD") ?: ""
} else if (sharedSigningFile.exists()) {
    keystoreProperties.load(FileInputStream(sharedSigningFile))
}

android {
    namespace = "dev.minifrost.minifrost_signer"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = "27.2.12479018" // installed NDK; flutter default (28.x) may not be

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "dev.minifrost.minifrost_signer"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = 26 // AAudio (data over sound) needs API 26
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    signingConfigs {
        if (hasReleaseKey) {
            create("release") {
                keyAlias = keystoreProperties["keyAlias"] as String
                keyPassword = keystoreProperties["keyPassword"] as String
                storeFile = file(keystoreProperties["storeFile"] as String)
                storePassword = keystoreProperties["storePassword"] as String
            }
        }
    }

    buildTypes {
        debug {
            // Debug builds are signed with the release key when it is available, so locally
            // installed debug and release builds share one signature and can update each other.
            signingConfig =
                if (hasReleaseKey) {
                    signingConfigs.getByName("release")
                } else {
                    signingConfigs.getByName("debug")
                }
        }
        release {
            // Use the real release key when one is configured, otherwise the debug key so
            // `flutter run --release` and local builds still work.
            signingConfig =
                if (hasReleaseKey) {
                    signingConfigs.getByName("release")
                } else {
                    signingConfigs.getByName("debug")
                }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}


flutter {
    source = "../.."
}
