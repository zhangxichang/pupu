import com.android.build.api.variant.AndroidComponentsExtension
import com.android.ide.common.vectordrawable.Svg2Vector
import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import java.io.ByteArrayOutputStream
import java.io.File

abstract class GenerateLauncherIconTask : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.NONE)
    abstract val iconSvg: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val svg = iconSvg.get().asFile
        val converted = ByteArrayOutputStream().use { output ->
            val error = Svg2Vector.parseSvgToXml(svg.toPath(), output)
            check(error.isBlank()) { "Svg2Vector failed for ${svg.name}: $error" }
            output.toString(Charsets.UTF_8.name())
        }
        check(converted.contains("<vector")) { "Svg2Vector produced no VectorDrawable for ${svg.name}" }

        val foregroundXml = converted
            .replace(Regex("""android:width="[^"]+""""), """android:width="108dp"""")
            .replace(Regex("""android:height="[^"]+""""), """android:height="108dp"""")
            .trimStart()
            .let { xml ->
                if (xml.startsWith("<?xml")) xml else "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n$xml"
            }

        val out = outputDir.get().asFile
        out.deleteRecursively()
        val drawable = File(out, "drawable").apply { mkdirs() }
        val mipmap = File(out, "mipmap-anydpi").apply { mkdirs() }

        File(drawable, "ic_launcher_background.xml").writeText(
            """
            <?xml version="1.0" encoding="utf-8"?>
            <vector xmlns:android="http://schemas.android.com/apk/res/android"
                android:width="108dp"
                android:height="108dp"
                android:viewportWidth="108"
                android:viewportHeight="108">
                <path
                    android:fillColor="#FFFFFF"
                    android:pathData="M0,0h108v108h-108z" />
            </vector>
            """.trimIndent() + "\n",
        )

        File(drawable, "ic_launcher_foreground.xml").writeText(foregroundXml)

        val adaptiveIcon = """
            <?xml version="1.0" encoding="utf-8"?>
            <adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
                <background android:drawable="@drawable/ic_launcher_background" />
                <foreground android:drawable="@drawable/ic_launcher_foreground" />
                <monochrome android:drawable="@drawable/ic_launcher_foreground" />
            </adaptive-icon>
        """.trimIndent() + "\n"
        File(mipmap, "ic_launcher.xml").writeText(adaptiveIcon)
        File(mipmap, "ic_launcher_round.xml").writeText(adaptiveIcon)

        logger.lifecycle("Generated launcher icon resources in ${out.absolutePath}")
    }
}

val generateLauncherIcon = tasks.register<GenerateLauncherIconTask>("generateLauncherIcon") {
    group = "build"
    description = "Generate Android launcher icon resources from icon.svg"
    iconSvg.set(rootProject.file("../../icon.svg"))
}

pluginManager.withPlugin("com.android.application") {
    val androidComponents = extensions.getByName("androidComponents")
        as AndroidComponentsExtension<*, *, *>
    androidComponents.onVariants { variant ->
        variant.sources.res?.addGeneratedSourceDirectory(
            generateLauncherIcon,
            GenerateLauncherIconTask::outputDir,
        )
    }
}
