package app.musicopy.ui.screens

import app.musicopy.mockNodeId
import io.kotest.assertions.json.shouldEqualJson
import io.kotest.core.spec.style.FunSpec
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemModel

class PreTransferScreenTest : FunSpec({
    context("buildTree") {
        test("builds a tree from paths") {
            val items = makeIndexItems(
                "library" to "album1/song1.mp3",
                "library" to "album1/song2.mp3",
                "library" to "album2/song1.mp3",
                "library" to "album2/song2.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodesToJsonString(tree).shouldEqualJson("""{
                "album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "album2": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }""")
        }

        test("removes shared leading paths") {
            val items = makeIndexItems(
                "library" to "shared/album1/song1.mp3",
                "library" to "shared/album1/song2.mp3",
                "library" to "shared/album2/song1.mp3",
                "library" to "shared/album2/song2.mp3",
                "library" to "shared/other.mp3"
            )

            val tree = buildTree(items)

            nodesToJsonString(tree).shouldEqualJson("""{
                "album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "album2": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }""")
        }

        test("collapses an artist with a single album") {
            val items = makeIndexItems(
                "library" to "artist1/album1/song1.mp3",
                "library" to "artist1/album1/song2.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodesToJsonString(tree).shouldEqualJson("""{
                "artist1/album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }""")
        }

        test("doesn't collapse an artist with multiple albums") {
            val items = makeIndexItems(
                "library" to "artist1/album1/song1.mp3",
                "library" to "artist1/album1/song2.mp3",
                "library" to "artist1/album2/song1.mp3",
                "library" to "artist1/album2/song2.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodesToJsonString(tree).shouldEqualJson("""{
                "artist1": {
                    "album1": {
                        "song1.mp3": true,
                        "song2.mp3": true
                    },
                    "album2": {
                        "song1.mp3": true,
                        "song2.mp3": true
                    }
                },
                "other.mp3": true
            }""")
        }

        test("doesn't collapse an artist with loose songs") {
            val items = makeIndexItems(
                "library" to "artist1/album1/song1.mp3",
                "library" to "artist1/album1/song2.mp3",
                "library" to "artist1/single.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodesToJsonString(tree).shouldEqualJson("""{
                "artist1": {
                    "album1": {
                        "song1.mp3": true,
                        "song2.mp3": true
                    },
                    "single.mp3": true
                },
                "other.mp3": true
            }""")
        }
    }
})

private fun nodeToJson(node: TreeNode): JsonElement {
    return node.leaf?.let {
        JsonPrimitive(true)
    } ?: run {
        nodesToJson(node.children)
    }
}

private fun nodesToJson(nodes: List<TreeNode>): JsonElement {
    return buildJsonObject {
        nodes.forEach {
            put(it.part, nodeToJson(it))
        }
    }
}

private fun nodesToJsonString(subject: List<TreeNode>): String {
    val json = Json {
        prettyPrint = true
    }
    return json.encodeToString(nodesToJson(subject))
}

private val nodeId = mockNodeId()

private fun makeIndexItems(vararg paths: Pair<String, String>): List<IndexItemModel> {
    return paths.asList().map { path ->
        IndexItemModel(
            nodeId = nodeId,
            root = path.first,
            path = path.second,
            fileSize = FileSizeModel.Unknown,
            downloaded = false
        )
    }
}
