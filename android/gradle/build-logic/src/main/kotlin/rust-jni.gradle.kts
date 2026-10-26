// Builds Rust libandroid.so for each Android variant and adds it to AGP's jniLibs source set.
import com.android.build.api.variant.AndroidComponentsExtension
import org.gradle.api.DefaultTask
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import java.io.File
import javax.inject.Inject

abstract class BuildRustJniLibsTask : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val rustSources: ConfigurableFileCollection

    @get:Input
    abstract val release: Property<Boolean>

    @get:Input
    abstract val abi: Property<String>

    @get:Input
    abstract val platform: Property<Int>

    @get:Input
    abstract val packageName: Property<String>

    @get:Input
    abstract val libName: Property<String>

    @get:Internal
    abstract val workingDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val execOperations: ExecOperations

    @TaskAction
    fun build() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        out.mkdirs()

        val command = mutableListOf(
            "cargo",
            "ndk",
            "-t",
            abi.get(),
            "-P",
            platform.get().toString(),
            "-o",
            out.absolutePath,
            "build",
            "-p",
            packageName.get(),
        )
        if (release.get()) {
            command += "--release"
        }

        val projectDir = workingDir.get().asFile
        logger.lifecycle(
            "Building ${abi.get()} for Android API ${platform.get()} " +
                "(${if (release.get()) "release" else "debug"})",
        )
        execOperations.exec {
            workingDir(projectDir)
            commandLine(command)
        }

        val library = File(out, "${abi.get()}/lib${libName.get()}.so")
        check(library.isFile) {
            "cargo ndk did not produce ${library.absolutePath}"
        }
        logger.lifecycle("Built ${library.relativeTo(projectDir)}")
    }
}

pluginManager.withPlugin("com.android.application") {
    val repoRoot = rootProject.projectDir.parentFile.parentFile
    val rustSourceFiles = rootProject.fileTree(repoRoot) {
        include("**/*.rs", "**/*.slint", "**/Cargo.toml", "**/Cargo.lock")
        exclude("android/gradle/**", "**/target/**", "**/build/**", ".git/**")
    }

    val androidComponents = extensions.getByName("androidComponents")
        as AndroidComponentsExtension<*, *, *>
    androidComponents.onVariants { variant ->
        val variantName = variant.name.replaceFirstChar { it.uppercase() }
        val buildRust = tasks.register<BuildRustJniLibsTask>("buildRustJniLibs$variantName") {
            group = "build"
            description = "Build Rust libandroid.so for the ${variant.name} variant"
            rustSources.from(rustSourceFiles)
            release.set(variant.buildType == "release")
            abi.set("arm64-v8a")
            platform.set(variant.minSdk.apiLevel)
            packageName.set("android")
            libName.set("android")
            workingDir.set(repoRoot)
            outputDir.set(layout.buildDirectory.dir("generated/jniLibs/${variant.name}"))
        }

        variant.sources.jniLibs?.addGeneratedSourceDirectory(
            buildRust,
            BuildRustJniLibsTask::outputDir,
        )
    }
}
