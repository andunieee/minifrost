import java.io.File
import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
}

val profile = (findProperty("profile") as String?) ?: "debug"
val abis = ((findProperty("abis") as String?) ?: "arm64-v8a").split(",")
val repo = rootDir.parentFile.parentFile.parentFile

// libminifrost_signer_app.so: the Freya UI and the engine, built by cargo-ndk.
val buildNative = tasks.register<Exec>("buildNative") {
    workingDir = repo
    commandLine(listOf(repo.resolve("scripts/android-native.sh").path, "signer", profile) + abis)
    inputs.files(
        fileTree(repo.resolve("crates")) { exclude("**/target/**") },
        fileTree(repo.resolve("apps/signer/src")),
        repo.resolve("Cargo.toml"), repo.resolve("Cargo.lock"), repo.resolve("apps/signer/Cargo.toml"),
    )
    outputs.dir("src/main/jniLibs")
}
tasks.matching { it.name == "preBuild" }.configureEach { dependsOn(buildNative) }

// Release signing is optional so local builds keep working. The release key is resolved from, in
// order of precedence:
//   1. android/key.properties, written by CI from the Android signing secrets;
//   2. ANDROID_KEYSTORE / ANDROID_KEYSTORE_PASSWORD / ANDROID_KEY_ALIAS / ANDROID_KEY_PASSWORD
//      (see ~/android-release/env.sh);
//   3. the shared ~/android-release/signing.properties used by local projects.
// Without any of them the builds are signed with the debug key.
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
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {
        applicationId = "dev.minifrost.minifrost_signer"
        // Freya's skia and AAudio (data over sound) need API 26, see scripts/android-native.sh.
        minSdk = 26
        targetSdk = 35
        // CI passes the release version and the run number; local builds take them from Cargo.toml.
        versionName = (findProperty("versionName") as String?)
            ?: Regex("""(?m)^version = "([^"]+)"""").find(repo.resolve("apps/signer/Cargo.toml").readText())!!.groupValues[1]
        versionCode = ((findProperty("versionCode") as String?) ?: "1").toInt()
        ndk { abiFilters += abis }
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
        // Debug builds are signed with the release key too when it is available, so locally installed
        // debug and release builds share one signature and can update each other.
        debug {
            signingConfig = signingConfigs.getByName(if (hasReleaseKey) "release" else "debug")
        }
        release {
            signingConfig = signingConfigs.getByName(if (hasReleaseKey) "release" else "debug")
            isMinifyEnabled = false
        }
    }

    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
    // MinifrostActivity, QrCamera and the natives' class, shared by both apps.
    sourceSets["main"].java.srcDirs(repo.resolve("crates/minifrost_ui/android"))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // The window insets.
    implementation("androidx.core:core:1.15.0")
}
