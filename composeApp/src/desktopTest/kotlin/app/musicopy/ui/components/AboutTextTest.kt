package app.musicopy.ui.components

import androidx.compose.ui.text.buildAnnotatedString
import io.kotest.core.spec.style.FunSpec
import io.kotest.matchers.shouldBe

class AboutTextTest : FunSpec({
    context("AnnotatedString.trimAnnotated") {
        test("removes leading and trailing whitespace") {
            val original = buildAnnotatedString {
                append("  foo  ")
            }

            val trimmed = original.trimAnnotated()

            trimmed.text shouldBe "foo"
        }

        test("removes trailing newline") {
            val original = buildAnnotatedString {
                appendLine("foo")
            }

            val trimmed = original.trimAnnotated()

            trimmed.text shouldBe "foo"
        }
    }
})
