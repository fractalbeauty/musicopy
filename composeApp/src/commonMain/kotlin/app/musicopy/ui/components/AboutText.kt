package app.musicopy.ui.components

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import kotlinx.datetime.format
import kotlinx.datetime.format.DateTimeComponents
import kotlinx.datetime.format.MonthNames
import kotlinx.datetime.format.char
import musicopy_root.musicopy.BuildConfig
import kotlin.text.appendLine
import kotlin.time.ExperimentalTime
import kotlin.time.Instant

@OptIn(ExperimentalTime::class)
@Composable
fun aboutText(
    supportText: Boolean = true,
): AnnotatedString = buildAnnotatedString {
    withUrl(
        "https://musicopy.app/manual",
    ) {
        append("User Manual")
    }
    append("  ⋅  ")
    withUrl(
        "https://github.com/fractalbeauty/musicopy",
    ) {
        append("Source")
    }
    appendLine()

    appendLine()

    val buildTime = Instant.fromEpochMilliseconds(BuildConfig.BUILD_TIME)
    val buildDate = buildTime.format(DateTimeComponents.Format {
        monthName(MonthNames.ENGLISH_FULL)
        char(' ')
        day()
        chars(", ")
        year()
    })
    appendLine("Version ${BuildConfig.APP_VERSION}, built on $buildDate.")

    appendLine()

    append(
        "Musicopy is available under the "
    )
    withUrl(
        "https://github.com/fractalbeauty/musicopy/blob/main/LICENSE",
    ) {
        append("GNU AGPLv3")
    }
    appendLine(".")

    appendLine()

    append("For more information, visit ")
    withUrl("https://musicopy.app") {
        append("musicopy.app")
    }
    appendLine(
        "."
    )

    if (supportText) {
        appendLine()

        appendSupportText()
    }
}.trimAnnotated()

@Composable
private fun AnnotatedString.Builder.appendSupportText() {
    append("For support, email ")
    withUrl("mailto:support@musicopy.app") {
        append("support@musicopy.app")
    }
    appendLine(".")
}

@Composable
internal fun AnnotatedString.Builder.withUrl(
    url: String,
    content: AnnotatedString.Builder.() -> Unit,
) {
    withLink(
        LinkAnnotation.Url(
            url = url,
            styles = TextLinkStyles(
                style = SpanStyle(
                    color = MaterialTheme.colorScheme.primary,
                    textDecoration = TextDecoration.Underline
                )
            )
        )
    ) {
        content()
    }
}

internal fun AnnotatedString.trimAnnotated(): AnnotatedString {
    return this.subSequence(
        startIndex = this.text.indexOfFirst { !it.isWhitespace() },
        endIndex = this.text.indexOfLast { !it.isWhitespace() } + 1
    )
}
