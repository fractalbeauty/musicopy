package app.musicopy.ui

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import musicopy_root.musicopy.generated.resources.InterVariable
import musicopy_root.musicopy.generated.resources.REM_Variable
import musicopy_root.musicopy.generated.resources.Res
import org.jetbrains.compose.resources.Font
import zip.meows.musicopy.ui.darkScheme
import zip.meows.musicopy.ui.lightScheme

@Composable
fun Theme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = appColorScheme(),
        typography = appTypography(),
        content = content,
    )
}

@Composable
fun appColorScheme(
//    darkTheme: Boolean = isSystemInDarkTheme,
    darkTheme: Boolean = false,
): ColorScheme =
    if (darkTheme) {
        darkScheme
    } else {
        lightScheme
    }

@Composable
fun appTypography(): Typography {
    val inter = interFontFamily()

    return Typography(
        displayLarge = MaterialTheme.typography.displayLarge.copy(fontFamily = inter),
        displayMedium = MaterialTheme.typography.displayMedium.copy(fontFamily = inter),
        displaySmall = MaterialTheme.typography.displaySmall.copy(fontFamily = inter),
        headlineLarge = MaterialTheme.typography.headlineLarge.copy(fontFamily = inter),
        headlineMedium = MaterialTheme.typography.headlineMedium.copy(fontFamily = inter),
        headlineSmall = MaterialTheme.typography.headlineSmall.copy(fontFamily = inter),
        titleLarge = MaterialTheme.typography.titleLarge.copy(fontFamily = inter),
        titleMedium = MaterialTheme.typography.titleMedium.copy(fontFamily = inter),
        titleSmall = MaterialTheme.typography.titleSmall.copy(fontFamily = inter),
        bodyLarge = MaterialTheme.typography.bodyLarge.copy(fontFamily = inter),
        bodyMedium = MaterialTheme.typography.bodyMedium.copy(fontFamily = inter),
        bodySmall = MaterialTheme.typography.bodySmall.copy(fontFamily = inter),
        labelLarge = MaterialTheme.typography.labelLarge.copy(fontFamily = inter),
        labelMedium = MaterialTheme.typography.labelMedium.copy(fontFamily = inter),
        labelSmall = MaterialTheme.typography.labelSmall.copy(fontFamily = inter),
    )
}

val Typography.logotype: TextStyle
    @Composable
    get() {
        return TextStyle(
            fontFamily = remFontFamily(),
            fontWeight = FontWeight.Bold,
            fontStyle = FontStyle.Normal,
            fontSize = 26.sp,
        )
    }

val Typography.widgetHeadline: TextStyle
    @Composable
    get() {
        return TextStyle(
            fontFamily = remFontFamily(),
            fontWeight = FontWeight.Normal,
            fontSize = 16.sp,
        )
    }

val Typography.monospaceMedium: TextStyle
    @Composable
    get() = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace)

@Composable
fun interFontFamily(): FontFamily = FontFamily(Font(Res.font.InterVariable))

@Composable
fun remFontFamily(): FontFamily = FontFamily(Font(Res.font.REM_Variable))
