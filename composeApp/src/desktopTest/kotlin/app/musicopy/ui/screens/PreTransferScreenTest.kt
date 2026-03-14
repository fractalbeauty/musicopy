package app.musicopy.ui.screens

import app.musicopy.mockNodeId
import io.kotest.assertions.json.shouldEqualJson
import io.kotest.core.spec.style.FunSpec
import io.kotest.matchers.shouldBe
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemDownloadStatusModel
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

            nodeToJsonString(tree) shouldEqualJson """{
                "album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "album2": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }"""
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

            nodeToJsonString(tree) shouldEqualJson """{
                "album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "album2": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }"""
        }

        test("collapses an artist with a single album") {
            val items = makeIndexItems(
                "library" to "artist1/album1/song1.mp3",
                "library" to "artist1/album1/song2.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodeToJsonString(tree) shouldEqualJson """{
                "artist1/album1": {
                    "song1.mp3": true,
                    "song2.mp3": true
                },
                "other.mp3": true
            }"""
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

            nodeToJsonString(tree) shouldEqualJson """{
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
            }"""
        }

        test("doesn't collapse an artist with loose songs") {
            val items = makeIndexItems(
                "library" to "artist1/album1/song1.mp3",
                "library" to "artist1/album1/song2.mp3",
                "library" to "artist1/single.mp3",
                "library" to "other.mp3"
            )

            val tree = buildTree(items)

            nodeToJsonString(tree) shouldEqualJson """{
                "artist1": {
                    "album1": {
                        "song1.mp3": true,
                        "song2.mp3": true
                    },
                    "single.mp3": true
                },
                "other.mp3": true
            }"""
        }

        test("sorts items alphabetically") {
            val items = makeIndexItems(
                "library" to "ddd.mp3",
                "library" to "aaa.mp3",
                "library" to "ccc.mp3",
                "library" to "bbb.mp3"
            )

            val tree = buildTree(items)

            nodeToJsonString(tree) shouldEqualJson """{
                "aaa.mp3": true,
                "bbb.mp3": true,
                "ccc.mp3": true,
                "ddd.mp3": true
            }"""
        }

        test("sorts folders before files") {
            val items = makeIndexItems(
                "library" to "zzz.mp3",
                "library" to "aaa-folder/song.mp3",
                "library" to "aaa.mp3",
                "library" to "zzz-folder/song.mp3"
            )

            val tree = buildTree(items)

            nodeToJsonString(tree) shouldEqualJson """{
                "aaa-folder": {
                    "song.mp3": true
                },
                "zzz-folder": {
                    "song.mp3": true
                },
                "aaa.mp3": true,
                "zzz.mp3": true
            }"""
        }
    }

    context("SelectionManager") {
        test("preselects Waiting items when paused") {
            val paused = true
            val manager = SelectionManager()

            // A and B are waiting, so they should be preselected
            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/b", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/c", IndexItemDownloadStatusModel.DOWNLOADED),
                    makeIndexItem("library", "/d", null),
                ),
                paused
            )

            manager.selectedKeys shouldBe setOf("library" to "/a", "library" to "/b")
        }

        test("does not preselect Waiting items when not paused") {
            val paused = false
            val manager = SelectionManager()

            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/b", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/c", IndexItemDownloadStatusModel.DOWNLOADED),
                    makeIndexItem("library", "/d", null),
                ),
                paused
            )

            manager.selectedKeys shouldBe emptySet()
        }

        test("preselects new Waiting items after refresh when paused") {
            val paused = true
            val manager = SelectionManager()

            // A is initially preselected
            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/b", null),
                ),
                paused
            )

            // Refresh changes status of B to Waiting
            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING),
                    makeIndexItem("library", "/b", IndexItemDownloadStatusModel.WAITING),
                ),
                paused
            )

            // Both should be selected
            manager.selectedKeys shouldBe setOf("library" to "/a", "library" to "/b")
        }

        test("doesn't re-preselect manually deselected Waiting items") {
            val manager = SelectionManager()

            val paused = true

            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING)
                ),
                paused
            )

            // User manually deselects A
            manager.setSelected(
                makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING),
                false,
                paused
            )

            // Refresh with same item still Waiting
            manager.updateSelection(
                listOf(
                    makeIndexItem("library", "/a", IndexItemDownloadStatusModel.WAITING)
                ),
                paused
            )

            // A should not be selected
            manager.selectedKeys shouldBe emptySet()
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

private fun nodeToJsonString(root: TreeNode): String {
    val json = Json {
        prettyPrint = true
    }
    return json.encodeToString(nodeToJson(root))
}

private val nodeId = mockNodeId()

private fun makeIndexItems(vararg paths: Pair<String, String>): List<IndexItemModel> {
    return paths.asList().map { path ->
        makeIndexItem(
            root = path.first,
            path = path.second,
            downloadStatus = null,
        )
    }
}

private fun makeIndexItem(
    root: String,
    path: String,
    downloadStatus: IndexItemDownloadStatusModel?,
): IndexItemModel {
    return IndexItemModel(
        nodeId = nodeId,
        root = root,
        path = path,
        downloadStatus = downloadStatus,
        fileSize = FileSizeModel.Unknown,
    )
}
