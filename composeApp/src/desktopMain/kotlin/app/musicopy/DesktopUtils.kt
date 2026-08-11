package app.musicopy

import java.awt.Desktop
import java.io.File

fun openDirectoryInExplorer(path: String) {
    val file = File(path)
    val target = if (file.isDirectory) file else File(file.parent)
    Desktop.getDesktop().open(target)
}
