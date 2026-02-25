package app.musicopy.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.triStateToggleable
import androidx.compose.material3.BottomAppBar
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.mutableStateSetOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshots.SnapshotStateList
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Fill
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.state.ToggleableState
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.BackHandler
import app.musicopy.formatSize
import app.musicopy.mockClientModel
import app.musicopy.ui.components.TopBar
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_downward_24px
import musicopy_root.musicopy.generated.resources.chevron_forward_24px
import musicopy_root.musicopy.generated.resources.exclamation_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.ClientModel
import uniffi.musicopy.DownloadPartialItemModel
import uniffi.musicopy.FileSizeModel
import uniffi.musicopy.IndexItemDownloadStatusModel
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
    val selectionManager = remember { SelectionManager() }
    LaunchedEffect(clientModel.index) {
        clientModel.index?.let { index ->
            selectionManager.onIndexChanged(index)
        }
    }

    // Build node graph
    val root = remember(clientModel.index) {
        buildTree(clientModel.index ?: emptyList())
    }

    // Build node size lookup
    val nodeSizes = remember(root) {
        buildNodeSizes(listOf(root))
    }

    // Total size from root node
    val rootSizeModel = nodeSizes.getOrElse(root) { FileSizeModel.Unknown }
    val totalSize = rootSizeModel.value()
    val totalSizeEstimated = rootSizeModel !is FileSizeModel.Actual

    // Navigation stack for breadcrumb navigation
    val navigationStack = remember { mutableStateListOf<String>() }
    val currentNode: TreeNode = navigationStack.fold(root) { node, part ->
        node.children.find { it.part == part } ?: node
    }

    BackHandler(enabled = navigationStack.isNotEmpty()) {
        navigationStack.removeAt(navigationStack.lastIndex)
    }

    // Stored scroll states for each folder
    val scrollStates = remember { mutableMapOf<String, LazyListState>() }

    // Current scroll state based on navigation stack
    val currentScrollState = scrollStates.getOrPut(navigationStack.joinToString("/")) {
        LazyListState()
    }

    // Current children and folder size
    val currentChildren = currentNode.children
    val folderSizeModel = nodeSizes.getOrElse(currentNode) { FileSizeModel.Unknown }
    val currentFolderSize = folderSizeModel.value()
    val currentFolderSizeEstimated = folderSizeModel !is FileSizeModel.Actual

    val onDownload = {
        val selectedKeys = selectionManager.selectedKeys
        val allSelected = selectedKeys.size == clientModel.index?.size

        if (selectedKeys.isEmpty() || allSelected) {
            onDownloadAll()
        } else {
            onDownloadPartial(selectedKeys.map { (root, path) ->
                DownloadPartialItemModel(
                    nodeId = clientModel.nodeId,
                    root = root,
                    path = path
                )
            })
        }
    }

    Scaffold(
        topBar = {
            TopBar(
                title = "Transfer",
                onShowNodeStatus = onShowNodeStatus,
                onBack = {
                    if (navigationStack.isNotEmpty()) {
                        navigationStack.removeAt(navigationStack.lastIndex)
                    } else {
                        onCancel()
                    }
                }
            )
        },
        bottomBar = {
            BottomAppBar {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                ) {
                    if (!hasDownloadDirectory) {
                        ActionButton(
                            onClick = onPickDownloadDirectory,
                            text = "Choose download directory"
                        )
                    } else {
                        val selectedKeys = selectionManager.selectedKeys
                        val allSelected = selectedKeys.size == clientModel.index?.size
                        val numFiles = clientModel.index?.size ?: 0
                        ActionButton(
                            onClick = onDownload,
                            enabled = hasDownloadDirectory,
                            text = if (selectedKeys.isEmpty() || allSelected) {
                                "Download everything ($numFiles files, ${
                                    formatSize(
                                        totalSize,
                                        estimated = totalSizeEstimated,
                                        decimals = 0
                                    )
                                })"
                            } else {
                                // Look up selected items from current index
                                val selectedItems = clientModel.index?.filter { item ->
                                    selectedKeys.contains(item.root to item.path)
                                } ?: emptyList()
                                val selectedSize =
                                    selectedItems.sumOf { item -> item.fileSize.value() }
                                val selectedEstimated =
                                    selectedItems.any { item -> item.fileSize !is FileSizeModel.Actual }

                                "Download selected (${selectedKeys.size} files, ${
                                    formatSize(
                                        selectedSize,
                                        estimated = selectedEstimated,
                                        decimals = 0
                                    )
                                })"
                            }
                        )
                    }
                }
            }
        },
        snackbarHost = snackbarHost,
    ) { innerPadding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(innerPadding),
        ) {
            BreadcrumbBar(
                navigationStack = navigationStack,
                currentNode = currentNode,
                deviceName = clientModel.name,
                onNavigateToRoot = { navigationStack.clear() },
                onNavigateToIndex = { index ->
                    // Keep items 0..index, remove rest
                    while (navigationStack.size > index + 1) {
                        navigationStack.removeAt(navigationStack.lastIndex)
                    }
                },
                checkboxRowState = selectionManager.getNodeState(currentNode),
                onCheckboxClick = { selectionManager.handleSelectNode(currentNode) },
                currentFolderSize = currentFolderSize,
                currentFolderSizeEstimated = currentFolderSizeEstimated,
            )

            LazyColumn(state = currentScrollState) {
                items(
                    items = currentChildren,
                    key = { node -> node.part }
                ) { node ->
                    FileRow(
                        node = node,
                        rowState = selectionManager.getNodeState(node),
                        onSelect = { selectionManager.handleSelectNode(node) },
                        onNavigate = if (node.leaf == null) {
                            { navigationStack.add(node.part) }
                        } else null,
                    )
                }
            }
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
private fun BreadcrumbBar(
    navigationStack: SnapshotStateList<String>,
    currentNode: TreeNode,
    deviceName: String,
    onNavigateToRoot: () -> Unit,
    onNavigateToIndex: (Int) -> Unit,
    checkboxRowState: RowState,
    onCheckboxClick: () -> Unit,
    currentFolderSize: ULong,
    currentFolderSizeEstimated: Boolean,
) {
    val scrollState = rememberScrollState()

    // Scroll to end when navigation stack changes
    LaunchedEffect(navigationStack.size) {
        if (navigationStack.isNotEmpty()) {
            scrollState.animateScrollTo(scrollState.maxValue)
        }
    }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.primaryContainer),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RowStateCheckbox(
            node = currentNode,
            // DisabledOrNone means nothing is actually selected; show unchecked at top level
            rowState = if (checkboxRowState == RowState.DisabledOrNone) RowState.None else checkboxRowState,
            onClick = onCheckboxClick,
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Row(
                modifier = Modifier
                    .weight(1f)
                    .horizontalScroll(scrollState),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = deviceName,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                    fontWeight = FontWeight.Bold,
                    modifier = if (navigationStack.isNotEmpty()) {
                        Modifier.clickable { onNavigateToRoot() }
                    } else {
                        Modifier
                    }
                )

                // Path crumbs
                navigationStack.forEachIndexed { index, part ->
                    Icon(
                        painter = painterResource(Res.drawable.chevron_forward_24px),
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onPrimaryContainer,
                        modifier = Modifier.padding(horizontal = 2.dp).requiredSize(18.dp)
                    )
                    Text(
                        text = part,
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onPrimaryContainer,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                        modifier = Modifier.clickable { onNavigateToIndex(index) }
                    )
                }

                // Add padding at the end for better scrolling
                if (navigationStack.isNotEmpty()) {
                    Box(modifier = Modifier.width(16.dp))
                }
            }

            Text(
                text = formatSize(
                    currentFolderSize,
                    estimated = currentFolderSizeEstimated,
                    decimals = 0,
                ),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
                modifier = Modifier.padding(start = 4.dp, end = 16.dp)
            )
        }
    }
}

@Composable
internal fun FileRow(
    node: TreeNode,
    rowState: RowState,
    onSelect: () -> Unit,
    onNavigate: (() -> Unit)?,
) {
    val isFolder = node.leaf == null

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(56.dp)
            .clickable(
                onClick = {
                    if (isFolder && onNavigate != null) {
                        onNavigate()
                    } else {
                        onSelect()
                    }
                },
            ),
        verticalAlignment = Alignment.CenterVertically
    ) {
        RowStateCheckbox(
            node = node,
            rowState = rowState,
            onClick = onSelect,
        )

        Row(
            modifier = Modifier.fillMaxSize().padding(end = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = node.part,
                style = MaterialTheme.typography.labelMedium,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f)
            )
            if (isFolder) {
                Icon(
                    painter = painterResource(Res.drawable.chevron_forward_24px),
                    contentDescription = "Navigate into folder"
                )
            }
        }
    }
    HorizontalDivider(thickness = 1.dp)
}

/**
 * Builds the graph of `TreeNodes` from the index.
 */
internal fun buildTree(
    index: List<IndexItemModel>,
): TreeNode {
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

    // strip single-child folders from the top
    var topLevel = roots
    while (topLevel.size == 1 && topLevel[0].leaf == null) {
        topLevel = topLevel[0].children
    }

    // sort top level nodes
    topLevel.sortWith(compareBy<TreeNode> { it.leaf != null }.thenBy { it.part })

    return TreeNode(part = "", children = topLevel)
}

/**
 * Collapses the children of a `TreeNode` recursively and sorts them.
 */
internal fun collapseNodeChildren(node: TreeNode) {
    // recursively collapse children first
    for (child in node.children) {
        collapseNodeChildren(child)
    }

    // sort children: folders first, then alphabetically
    node.children.sortWith(compareBy<TreeNode> { it.leaf != null }.thenBy { it.part })

    // duplicate list so we can safely iterate while modifying
    val oldChildren = node.children.toList()

    for (child in oldChildren) {
        // can't collapse leaves
        if (child.leaf != null) {
            continue
        }

        // only collapse if there's exactly one child and it's a folder
        val shouldCollapse = child.children.size == 1 && child.children[0].leaf == null
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
     * All descendants are disabled (downloaded or failed).
     */
    Disabled,

    /**
     * All descendants are disabled (downloaded or failed) or unselected.
     */
    DisabledOrNone,

    /**
     * All descendants are disabled (downloaded or failed) or selected.
     */
    DisabledOrSelected,

    /**
     * Some descendants are selected and some are unselected.
     *
     * Some descendants may also be disabled (downloaded or failed).
     */
    Indeterminate,
}

/**
 * Renders the appropriate checkbox for a given [RowState]:
 * - [RowState.Disabled]: a non-interactive [DisabledIconCheckbox]; uses [exclamation_24px] for
 *   [IndexItemDownloadStatusModel.FAILED] leaves, [arrow_downward_24px] otherwise
 * - All other states: a [TriStateCheckbox]
 */
@Composable
internal fun RowStateCheckbox(
    node: TreeNode,
    rowState: RowState,
    onClick: () -> Unit,
) {
    if (rowState == RowState.Disabled) {
        val painter = if (node.leaf?.downloadStatus == IndexItemDownloadStatusModel.FAILED) {
            painterResource(Res.drawable.exclamation_24px)
        } else {
            painterResource(Res.drawable.arrow_downward_24px)
        }
        DisabledIconCheckbox(painter = painter)
    } else {
        val toggleableState = when (rowState) {
            RowState.None -> ToggleableState.Off
            RowState.Selected -> ToggleableState.On
            RowState.DisabledOrNone -> ToggleableState.Indeterminate
            RowState.DisabledOrSelected -> ToggleableState.On
            RowState.Indeterminate -> ToggleableState.Indeterminate
            RowState.Disabled -> ToggleableState.Off
        }
        TriStateCheckbox(
            state = toggleableState,
            onClick = onClick,
        )
    }
}

private val CheckboxStateLayerSize = 40.dp
private val CheckboxDefaultPadding = 2.dp
private val CheckboxSize = 20.dp
private val StrokeWidth = 2.dp
private val RadiusSize = 2.dp

/**
 * Extracted M3 checkbox component with the check replaced by a given icon.
 * Disabled and non-interactive. Doesn't animate.
 */
@Composable
internal fun DisabledIconCheckbox(painter: Painter) {
    val toggleableModifier = Modifier.triStateToggleable(
        state = ToggleableState.On,
        onClick = {},
        enabled = false,
        role = Role.Checkbox,
        interactionSource = null,
        indication = ripple(
            bounded = false,
            radius = CheckboxStateLayerSize / 2
        )
    )

    val colors = CheckboxDefaults.colors()
    val boxColor = colors.disabledCheckedBoxColor
    val borderColor = colors.disabledBorderColor

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

        with(painter) {
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

/**
 * Manages selection state for PreTransferScreen.
 *
 * Automatically preselects InProgress items when the index changes, but only once per item.
 * If the user manually deselects an InProgress item, it won't be re-preselected on refresh.
 */
internal class SelectionManager {
    private val _selectedKeys = mutableStateSetOf<Pair<String, String>>()
    private val _preselectedKeys = mutableStateSetOf<Pair<String, String>>()
    private val _stateCache = mutableMapOf<TreeNode, RowState>()

    val selectedKeys: Set<Pair<String, String>> get() = _selectedKeys

    fun onIndexChanged(index: List<IndexItemModel>) {
        val toPreselect = index
            .filter { it.downloadStatus == IndexItemDownloadStatusModel.IN_PROGRESS }
            .map { it.root to it.path }
            .filterNot { _preselectedKeys.contains(it) }

        _selectedKeys.addAll(toPreselect)
        _preselectedKeys.addAll(toPreselect)
        _stateCache.clear()
    }

    /**
     * Sets the selected state of an item.
     */
    fun setSelected(item: IndexItemModel, selected: Boolean) {
        val key = item.root to item.path
        if (selected) {
            _selectedKeys.add(key)
        } else {
            _selectedKeys.remove(key)
        }
        _stateCache.clear()
    }

    /**
     * Sets the selected state of an item and its descendants.
     */
    fun setSelectedRecursive(node: TreeNode, selected: Boolean) {
        node.leaf?.let { setSelected(it, selected) }
        node.children.forEach { setSelectedRecursive(it, selected) }
    }

    /**
     * Gets whether an item is selected.
     */
    fun isSelected(item: IndexItemModel): Boolean {
        return _selectedKeys.contains(item.root to item.path)
    }

    /**
     * Handles selecting/deselecting a node based on its current state.
     * For leaf nodes, toggles selection. For branch nodes, recursively selects/deselects.
     */
    fun handleSelectNode(node: TreeNode) {
        node.leaf?.let { leaf ->
            setSelected(leaf, !isSelected(leaf))
        } ?: run {
            val rowState = getNodeState(node)
            when (rowState) {
                RowState.Selected, RowState.DisabledOrSelected, RowState.Indeterminate -> {
                    setSelectedRecursive(node, false)
                }

                RowState.None, RowState.DisabledOrNone -> {
                    setSelectedRecursive(node, true)
                }

                RowState.Disabled -> {}
            }
        }
    }

    /**
     * Gets the `RowState` of a node in the file tree.
     *
     * Uses an internal cache to avoid recomputing states for nodes.
     * The cache is cleared when selection changes.
     *
     * We need to know more than just Indeterminate to correctly
     * select/unselect indeterminate rows with mixed descendants.
     *
     * If the node is a leaf (file), then:
     *  - If it is downloaded or failed, the state is Disabled
     *  - If it is selected, the state is Selected
     *  - Otherwise, the state is None
     * If the node is a branch, then:
     *  - If it has no children, it is None
     *  - If all children are Disabled, it is Disabled
     *  - If all children are Selected, it is Selected
     *  - If all children are None, it is None
     *  - If all children are DisabledOrNone, Disabled, or None, it is DisabledOrNone
     *  - If all children are DisabledOrSelected, Disabled, or Selected, it is DisabledOrSelected
     *  - Otherwise, it is Indeterminate
     */
    fun getNodeState(node: TreeNode): RowState {
        _stateCache[node]?.let { return it }

        val state = node.leaf?.let {
            // leaf node
            if (it.downloadStatus == IndexItemDownloadStatusModel.DOWNLOADED ||
                it.downloadStatus == IndexItemDownloadStatusModel.FAILED
            ) {
                RowState.Disabled
            } else if (isSelected(it)) {
                RowState.Selected
            } else {
                RowState.None
            }
        } ?: run {
            // internal node
            if (node.children.isEmpty()) {
                return RowState.None
            }

            var total = 0
            var countNone = 0
            var countSelected = 0
            var countDisabled = 0
            var countDisabledOrNone = 0
            var countDisabledOrSelected = 0

            node.children.forEach { child ->
                val childState = getNodeState(child)
                when (childState) {
                    RowState.None -> {
                        total += 1
                        countNone += 1
                    }

                    RowState.Selected -> {
                        total += 1
                        countSelected += 1
                    }

                    RowState.Disabled -> {
                        total += 1
                        countDisabled += 1
                    }

                    RowState.DisabledOrNone -> {
                        total += 1
                        countDisabledOrNone += 1
                    }

                    RowState.DisabledOrSelected -> {
                        total += 1
                        countDisabledOrSelected += 1
                    }

                    RowState.Indeterminate -> {}
                }
            }

            if (countNone == total) {
                RowState.None
            } else if (countSelected == total) {
                RowState.Selected
            } else if (countDisabled == total) {
                RowState.Disabled
            } else if (countSelected == 0 && countDisabledOrSelected == 0) {
                RowState.DisabledOrNone
            } else if (countNone == 0 && countDisabledOrNone == 0) {
                RowState.DisabledOrSelected
            } else {
                RowState.Indeterminate
            }
        }

        _stateCache[node] = state
        return state
    }
}

internal data class TreeNode(
    val part: String,
    val children: MutableList<TreeNode> = mutableListOf(),
    val leaf: IndexItemModel? = null,
)

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
