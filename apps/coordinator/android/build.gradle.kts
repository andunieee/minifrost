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

// Plugin subprojects (e.g. jni via path_provider) default to the Flutter SDK's NDK, which isn't
// installed here (and falls back to a broken sdk ndk-bundle); use the same NDK as the app.
// Must come before the evaluationDependsOn block below so it is registered before they evaluate.
subprojects {
    if (name != "app") {
        afterEvaluate {
            extensions.findByType<com.android.build.gradle.BaseExtension>()?.ndkVersion = "27.2.12479018"
        }
    }
}

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}
subprojects {
    project.evaluationDependsOn(":app")
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
