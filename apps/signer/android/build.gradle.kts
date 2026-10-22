allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}
// plugins (e.g. jni) don't pin an NDK and AGP would pick the broken /opt/android-sdk/ndk-bundle
subprojects {
    afterEvaluate {
        extensions.findByName("android")?.let {
            (it as com.android.build.gradle.BaseExtension).ndkVersion = "27.2.12479018"
        }
    }
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
subprojects {
    project.evaluationDependsOn(":app")
}
