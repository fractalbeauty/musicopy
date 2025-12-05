package app.musicopy.ui.screens

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.selection.triStateToggleable
import androidx.compose.material3.Button
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TriStateCheckbox
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateSetOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshots.SnapshotStateSet
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Fill
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.AppSettings
import app.musicopy.formatSize
import app.musicopy.mockClientModel
import app.musicopy.ui.components.DetailBox
import app.musicopy.ui.components.DetailItem
import app.musicopy.ui.components.SectionHeader
import app.musicopy.ui.components.TopBar
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_downward_24px
import musicopy_root.musicopy.generated.resources.chevron_forward_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import uniffi.musicopy.DownloadPartialItemModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemModel
import kotlin.math.floor
import kotlin.math.max

@Composable
fun PreTransferScreen(
    snackbarHost: @Composable () -> Unit,
    onShowNodeStatus: () -> Unit,

    clientModel: ClientModel,
    hasDownloadDirectory: Boolean,
    onPickDownloadDirectory: () -> Unit,
    onDownloadAll: () -> Unit,
    onDownloadPartial: (List<DownloadPartialItemModel>) -> Unit,
    onCancel: () -> Unit,
) {
    val numFolders = remember(clientModel.index) {
        countIndexFolders(
            clientModel.index ?: emptyList()
        )
    }
    val numFiles = remember(clientModel.index) {
        clientModel.index?.size ?: 0
    }
    val totalSize = remember(clientModel.index) {
        clientModel.index?.let { index ->
            index.sumOf { item -> item.fileSize.value() }
        } ?: 0u
    }
    val totalSizeEstimated = remember(clientModel.index) {
        clientModel.index?.let { index ->
            index.any { it.fileSize !is FileSizeModel.Actual }
        } ?: false
    }

    val selected = remember { mutableStateSetOf<IndexItemModel>() }

    val allCheckboxState = if (selected.isEmpty()) {
        ToggleableState.Off
    } else {
        ToggleableState.On
    }
    val onAllCheckboxClick: () -> Unit = {
        if (selected.isEmpty()) {
            clientModel.index?.let { index ->
                selected.clear()
                selected.addAll(index)
            }
        } else {
            selected.clear()
        }
    }

    val onDownload = {
        val allSelected = selected.size == clientModel.index?.size

        if (selected.isEmpty() || allSelected) {
            onDownloadAll()
        } else {
            onDownloadPartial(selected.map { item ->
                DownloadPartialItemModel(
                    nodeId = item.nodeId,
                    root = item.root,
                    path = item.path
                )
            })
        }
    }

    Scaffold(
        topBar = {
            TopBar(
                title = "Transfer",
                onShowNodeStatus = onShowNodeStatus,
                onBack = onCancel
            )
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
        ) {
            Column(
                modifier = Modifier.padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                DetailBox {
                    DetailItem("Folders", "$numFolders")
                    DetailItem("Files", "$numFiles")
                    DetailItem(
                        "Total Size",
                        formatSize(
                            totalSize,
                            estimated = totalSizeEstimated,
                            decimals = 0,
                        )
                    )
                }

                if (!hasDownloadDirectory) {
                    ActionButton(
                        onClick = onPickDownloadDirectory,
                        text = "Choose download directory"
                    )
                }

                val allSelected = selected.size == clientModel.index?.size
                ActionButton(
                    onClick = onDownload,
                    enabled = hasDownloadDirectory,
                    text = if (selected.isEmpty() || allSelected) {
                        "Download everything"
                    } else {
                        val selectedSize = selected.sumOf { item -> item.fileSize.value() }
                        val selectedEstimated =
                            selected.any { item -> item.fileSize !is FileSizeModel.Actual }

                        "Download selected (${selected.size} files, ${
                            formatSize(
                                selectedSize,
                                estimated = selectedEstimated,
                                decimals = 0
                            )
                        })"
                    }
                )
            }

            HorizontalDivider(thickness = 1.dp)

            SectionHeader(
                text = "FILES",
                leftContent = {
                    TriStateCheckbox(
                        state = allCheckboxState,
                        onClick = onAllCheckboxClick
                    )
                },
                contentPadding = PaddingValues()
            )

            Tree(
                clientModel = clientModel,
                selected = selected,
            )
        }
    }
}

@Composable
private fun ActionButton(
    onClick: () -> Unit,
    enabled: Boolean = true,
    text: String,
) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.fillMaxWidth().height(64.dp),
        shape = MaterialTheme.shapes.large,
        contentPadding = PaddingValues(16.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = text,
            )

            Icon(
                painter = painterResource(Res.drawable.chevron_forward_24px),
                contentDescription = null,
            )
        }
    }
}

@Composable
internal fun Tree(
    clientModel: ClientModel,
    selected: SnapshotStateSet<IndexItemModel>,
) {
    // build node graph
    val topLevelNodes = remember(clientModel.index) {
        buildTree(clientModel.index ?: emptyList())
    }

    // build node size lookup
    val nodeSizes = remember(topLevelNodes) {
        buildNodeSizes(topLevelNodes)
    }

    val expanded = remember {
        val expanded = mutableStateListOf<TreeNode>()

        // expand top level nodes if they contain only non-leaves
        for (node in topLevelNodes) {
            if (node.children.all { child -> child.leaf == null }) {
                expanded.add(node)
            }
        }

        expanded
    }

    LazyColumn {
        topLevelNodes.forEach { topLevelNode ->
            renderNode(
                node = topLevelNode,
                isExpanded = { node ->
                    expanded.contains(node)
                },
                onExpand = { node ->
                    if (expanded.contains(node)) {
                        expanded.remove(node)
                    } else {
                        expanded.add(node)
                    }
                },
                isSelected = { item -> selected.contains(item) },
                onSelect = { item, shouldSelect ->
                    if (shouldSelect) {
                        selected.add(item)
                    } else {
                        selected.remove(item)
                    }
                },
                nodeSizes = nodeSizes
            )
        }
    }
}

/**
 * Builds the graph of `TreeNodes` from the index.
 *
 * Returns a list of top-level nodes.
 */
internal fun buildTree(
    index: List<IndexItemModel>,
): List<TreeNode> {
    val roots = mutableListOf<TreeNode>()

    // add nodes to tree
    for (item in index) {
        // find or create root
        val root = roots.find { node -> node.part == item.root } ?: run {
            val new = TreeNode(
                part = item.root,
            )
            roots.add(new)
            new
        }

        // split into path parts and filename
        val path = item.path.removePrefix("/")
        val parts = path.split('/')
        val lastPart = parts.last()
        val pathParts = parts.dropLast(1)

        // recursively find or create path nodes
        var curr = root
        for (part in pathParts) {
            val next = curr.children.find { node -> node.part == part } ?: run {
                val new = TreeNode(
                    part = part,
                )
                curr.children.add(new)
                new
            }
            curr = next
        }

        // create leaf node
        curr.children.add(
            TreeNode(
                part = lastPart,
                leaf = item
            )
        )
    }

    // collapse nodes with no loose files
    for (root in roots) {
        collapseNodeChildren(root)
    }

    // if there's only one root, return its contents as the top level
    return if (roots.size == 1) {
        roots[0].children
    } else {
        roots
    }
}

/**
 * Collapses the children of a `TreeNode` recursively.
 */
internal fun collapseNodeChildren(node: TreeNode) {
    // recursively collapse children first
    for (child in node.children) {
        collapseNodeChildren(child)
    }

    // duplicate list so we can safely iterate while modifying
    val oldChildren = node.children.toList()

    for (child in oldChildren) {
        // can't collapse leaves
        if (child.leaf != null) {
            continue;
        }

        // only collapse if all grandchildren are non-leafs
        val shouldCollapse = child.children.all { grandchild -> grandchild.leaf == null }
        if (!shouldCollapse) {
            continue
        }

        // find index to insert at
        val childIndex = node.children.indexOf(child)

        // add grandchildren with combined path to parent node
        // reverse iterator so the added nodes are in the correct order
        for (grandchild in child.children.reversed()) {
            val newNode = TreeNode(
                part = "${child.part}/${grandchild.part}",
                children = grandchild.children,
                leaf = grandchild.leaf,
            )
            node.children.add(childIndex, newNode)
        }

        // remove this node from the parent node
        node.children.remove(child)
    }
}

/**
 * Builds a map of sizes of TreeNodes.
 */
internal fun buildNodeSizes(
    nodes: List<TreeNode>,
    map: MutableMap<TreeNode, FileSizeModel> = mutableMapOf(),
): MutableMap<TreeNode, FileSizeModel> {
    for (node in nodes) {
        // recursively build sizes of children
        buildNodeSizes(node.children, map)

        // determine size of this node
        val size = node.leaf?.fileSize ?: run {
            // internal node's size is sum of child sizes
            val total = node.children.sumOf { child ->
                val childSize = map.getOrElse(
                    child,
                    defaultValue = { FileSizeModel.Unknown }
                )
                childSize.value()
            }

            // internal node is estimated if any child size is not actual
            val isEstimated = node.children.any { child ->
                val childSize = map.getOrElse(
                    child,
                    defaultValue = { FileSizeModel.Unknown }
                )
                childSize !is FileSizeModel.Actual
            }

            if (isEstimated) {
                FileSizeModel.Estimated(total)
            } else {
                FileSizeModel.Actual(total)
            }
        }

        // add to map
        map[node] = size
    }

    return map
}

internal enum class RowState {
    /**
     * All descendants are unselected.
     */
    None,

    /**
     * All descendants are selected.
     */
    Selected,

    /**
     * All descendants are downloaded.
     */
    Downloaded,

    /**
     * All descendants are downloaded or unselected.
     */
    DownloadedOrNone,

    /**
     * All descendants are downloaded or selected.
     */
    DownloadedOrSelected,

    /**
     * Some descendants are selected and some are unselected.
     *
     * Some descendants may also be downloaded.
     */
    Indeterminate,
}

/**
 * Gets the `RowState` of a node in the file tree.
 *
 * We need to know more than just Indeterminate to correctly
 * select/unselect indeterminate rows with mixed descendants.
 *
 * If the node is a leaf (file), then:
 *  - If it is downloaded, the state is Downloaded
 *  - If it is selected, the state is Selected
 *  - Otherwise, the state is None
 * If the node is a branch, then:
 *  - If it has no children, it is null
 *  - If all children are Downloaded, it is Downloaded
 *  - If all children are Selected, it is Selected
 *  - If all children are None, it is None
 *  - If all children are DownloadedOrNone, Downloaded, or None, it is DownloadedOrNone
 *  - If all children are DownloadedOrSelected, Downloaded, or Selected, it is DownloadedOrSelected
 *  - Otherwise, it is Indeterminate
 */
internal fun getNodeState(
    node: TreeNode,
    isSelected: (IndexItemModel) -> Boolean,
): RowState? {
    return node.leaf?.let {
        // leaf node
        if (it.downloaded) {
            RowState.Downloaded
        } else if (isSelected(it)) {
            RowState.Selected
        } else {
            RowState.None
        }
    } ?: run {
        // internal node
        if (node.children.isEmpty()) {
            return null
        }

        var total = 0
        var countNone = 0
        var countSelected = 0
        var countDownloaded = 0
        var countDownloadedOrNone = 0
        var countDownloadedOrSelected = 0

        node.children.forEach { child ->
            val state = getNodeState(child, isSelected)
            when (state) {
                RowState.None -> {
                    total += 1
                    countNone += 1
                }

                RowState.Selected -> {
                    total += 1
                    countSelected += 1
                }

                RowState.Downloaded -> {
                    total += 1
                    countDownloaded += 1
                }

                RowState.DownloadedOrNone -> {
                    total += 1
                    countDownloadedOrNone += 1
                }

                RowState.DownloadedOrSelected -> {
                    total += 1
                    countDownloadedOrSelected += 1
                }

                RowState.Indeterminate, null -> {}
            }
        }

        if (countNone == total) {
            RowState.None
        } else if (countSelected == total) {
            RowState.Selected
        } else if (countDownloaded == total) {
            RowState.Downloaded
        } else if (countSelected == 0 && countDownloadedOrSelected == 0) {
            RowState.DownloadedOrNone
        } else if (countNone == 0 && countDownloadedOrNone == 0) {
            RowState.DownloadedOrSelected
        } else {
            RowState.Indeterminate
        }
    }
}

/**
 * Calls `onSelect` on all leaf nodes including and below `node` with the value of `shouldSelect`.
 */
internal fun onSelectRecursive(
    node: TreeNode,
    onSelect: (IndexItemModel, Boolean) -> Unit,
    shouldSelect: Boolean,
) {
    node.leaf?.let {
        onSelect(it, shouldSelect)
    }

    node.children.forEach {
        onSelectRecursive(it, onSelect, shouldSelect)
    }
}

internal fun LazyListScope.renderNode(
    node: TreeNode,
    isExpanded: (TreeNode) -> Boolean,
    onExpand: (TreeNode) -> Unit,
    isSelected: (IndexItemModel) -> Boolean,
    onSelect: (IndexItemModel, Boolean) -> Unit,
    nodeSizes: Map<TreeNode, FileSizeModel>,
    keyPath: String = "",
    indent: Int = 0,
) {
    val rowState = getNodeState(node, isSelected)

    val onSelectThis = node.leaf?.let {
        {
            // toggle selected item
            onSelect(it, !isSelected(it))
        }
    } ?: run {
        {
            // set children based on current state
            when (rowState) {
                RowState.Selected, RowState.DownloadedOrSelected, RowState.Indeterminate -> {
                    onSelectRecursive(node, onSelect, false)
                }

                RowState.None, RowState.DownloadedOrNone -> {
                    onSelectRecursive(node, onSelect, true)
                }

                RowState.Downloaded, null -> {}
            }
        }
    }

    item(key = "$keyPath/${node.part}") {
        TreeRow(
            node,
            isExpanded = isExpanded(node),
            onExpand = { onExpand(node) },
            rowState = rowState,
            onSelect = onSelectThis,
            fileSize = nodeSizes.getOrElse(node, defaultValue = { FileSizeModel.Unknown }),
            indent = indent,
        )
    }

    if (isExpanded(node)) {
        node.children.forEach { child ->
            renderNode(
                node = child,
                isExpanded = isExpanded,
                onExpand = onExpand,
                indent = indent + 1,
                isSelected = isSelected,
                onSelect = onSelect,
                nodeSizes = nodeSizes,
                keyPath = "$keyPath/${node.part}"
            )
        }
    }
}

private val CheckboxStateLayerSize = 40.dp
private val CheckboxDefaultPadding = 2.dp
private val CheckboxSize = 20.dp
private val StrokeWidth = 2.dp
private val RadiusSize = 2.dp

/**
 * Extracted M3 checkbox component with the check replaced by a down arrow.
 * Doesn't animate.
 */
@Composable
internal fun DownloadedCheckbox() {
    val state = ToggleableState.On
    val enabled = false

    val toggleableModifier = Modifier.triStateToggleable(
        state = state,
        onClick = {},
        enabled = enabled,
        role = Role.Checkbox,
        interactionSource = null,
        indication = ripple(
            bounded = false,
            radius = CheckboxStateLayerSize / 2
        )
    )

    val colors = CheckboxDefaults.colors()
    val checkColor = colors.checkedCheckmarkColor
    val boxColor = colors.disabledCheckedBoxColor
    val borderColor = colors.disabledBorderColor

    val arrowPainter = painterResource(Res.drawable.arrow_downward_24px)

    Canvas(
        modifier = Modifier
            .minimumInteractiveComponentSize()
            .then(toggleableModifier)
            .padding(CheckboxDefaultPadding)
            .wrapContentSize(Alignment.Center)
            .requiredSize(CheckboxSize)
    ) {
        val strokeWidthPx = floor(StrokeWidth.toPx())
        drawBox(
            boxColor = boxColor,
            borderColor = borderColor,
            radius = RadiusSize.toPx(),
            strokeWidth = strokeWidthPx
        )

        with(arrowPainter) {
            draw(size)
        }
    }
}

private fun DrawScope.drawBox(
    boxColor: Color,
    borderColor: Color,
    radius: Float,
    strokeWidth: Float,
) {
    val halfStrokeWidth = strokeWidth / 2.0f
    val stroke = Stroke(strokeWidth)
    val checkboxSize = size.width
    if (boxColor == borderColor) {
        drawRoundRect(
            boxColor,
            size = Size(checkboxSize, checkboxSize),
            cornerRadius = CornerRadius(radius),
            style = Fill
        )
    } else {
        drawRoundRect(
            boxColor,
            topLeft = Offset(strokeWidth, strokeWidth),
            size = Size(checkboxSize - strokeWidth * 2, checkboxSize - strokeWidth * 2),
            cornerRadius = CornerRadius(max(0f, radius - strokeWidth)),
            style = Fill
        )
        drawRoundRect(
            borderColor,
            topLeft = Offset(halfStrokeWidth, halfStrokeWidth),
            size = Size(checkboxSize - strokeWidth, checkboxSize - strokeWidth),
            cornerRadius = CornerRadius(radius - halfStrokeWidth),
            style = stroke
        )
    }
}

@Composable
internal fun TreeRow(
    node: TreeNode,
    isExpanded: Boolean,
    onExpand: () -> Unit,
    rowState: RowState?,
    onSelect: () -> Unit,
    fileSize: FileSizeModel,
    indent: Int,
) {
    val degrees by animateFloatAsState(if (isExpanded) 90f else 0f)

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(56.dp)
            .clickable(
                onClick = {
                    if (node.children.isEmpty()) {
                        onSelect()
                    } else {
                        onExpand()
                    }
                },
                // should be clickable if not downloaded or not leaf
                enabled = (rowState != RowState.Downloaded) || (node.children.isNotEmpty())
            ),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Box(modifier = Modifier.width((indent * 24).dp))

        if (rowState == RowState.Downloaded) {
            DownloadedCheckbox()
        } else {
            val toggleableState = when (rowState) {
                RowState.None -> ToggleableState.Off
                RowState.Selected -> ToggleableState.On
                RowState.Downloaded -> ToggleableState.On
                RowState.DownloadedOrNone -> ToggleableState.Indeterminate
                RowState.DownloadedOrSelected -> ToggleableState.On
                RowState.Indeterminate -> ToggleableState.Indeterminate
                null -> ToggleableState.Off
            }
            val enabled = rowState != null

            TriStateCheckbox(
                state = toggleableState,
                enabled = enabled,
                onClick = onSelect,
            )
        }

        Row(
            modifier = Modifier
                .fillMaxSize(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = node.part,
                style = MaterialTheme.typography.bodyLarge,
                maxLines = 1,
                overflow = TextOverflow.StartEllipsis,
                modifier = Modifier.weight(1f)
            )

            node.leaf?.let { leaf ->
                // Text("${leaf.path}", modifier = Modifier.padding(end = 16.dp))
            } ?: run {
                Text(
                    formatSize(
                        fileSize.value(),
                        estimated = fileSize !is FileSizeModel.Actual,
                        decimals = 0,
                    ),
                    style = MaterialTheme.typography.labelLarge,
                    modifier = Modifier.padding(horizontal = 8.dp)
                )

                Image(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = "Expand icon",
                    modifier = Modifier.padding(end = 8.dp).rotate(degrees),
                    colorFilter = ColorFilter.tint(MaterialTheme.colorScheme.onSurface),
                )
            }
        }
    }
    HorizontalDivider(thickness = 1.dp)
}

internal data class TreeNode(
    val part: String,
    val children: MutableList<TreeNode> = mutableListOf(),
    val leaf: IndexItemModel? = null,
)

internal fun countIndexFolders(index: List<IndexItemModel>): Int {
    val seen = mutableSetOf<String>()

    for (item in index) {
        // split by / and drop last part
        val path = item.path.removePrefix("/")
        val parts = path.split('/')
        val pathParts = parts.dropLast(1)

        // count unique
        val key = pathParts.joinToString("/")
        seen.add(key)
    }

    return seen.size
}

fun FileSizeModel.value(): ULong {
    return when (this) {
        is FileSizeModel.Actual -> v1
        is FileSizeModel.Estimated -> v1
        is FileSizeModel.Unknown -> 0uL
    }
}

@Composable
fun PreTransferScreenSandbox() {
    var hasDownloadDirectory by remember { mutableStateOf((false)) }

    PreTransferScreen(
        snackbarHost = {},
        onShowNodeStatus = {},

        clientModel = mockClientModel(),
        hasDownloadDirectory = hasDownloadDirectory,
        onPickDownloadDirectory = { hasDownloadDirectory = true },
        onDownloadAll = {},
        onDownloadPartial = {},
        onCancel = { hasDownloadDirectory = false }
    )
}
