package app.musicopy.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun <T, K> AnimatedList(
    items: List<T>,
    itemKey: (T) -> K,
    enter: EnterTransition = fadeIn() + expandVertically(),
    exit: ExitTransition = fadeOut() + shrinkVertically(),
    render: @Composable (T) -> Unit,
) {
    val renderedItems =
        remember { mutableStateListOf<Pair<T, MutableState<Boolean>>>() }
    val coroutineScope = rememberCoroutineScope()

    LaunchedEffect(items, itemKey) {
        // animate out removed items
        val newKeys = items.map(itemKey).toSet()
        renderedItems.filter { itemKey(it.first) !in newKeys }.forEach { item ->
            coroutineScope.launch {
                item.second.value = false
            }
        }

        items.forEach { item ->
            // check if item is currently rendered
            val existingIndex = renderedItems.indexOfFirst { itemKey(it.first) == itemKey(item) }
            if (existingIndex == -1) {
                // add item
                val visible = mutableStateOf(false)
                renderedItems.add(Pair(item, visible))
                coroutineScope.launch {
                    delay(10)
                    visible.value = true
                }
            } else {
                // update item
                val existing = renderedItems[existingIndex]
                if (existing.first != item) {
                    renderedItems[existingIndex] = Pair(item, existing.second)
                }
            }
        }
    }

    renderedItems.forEach { item ->
        key(itemKey(item.first)) {
            val visible by item.second
            AnimatedVisibility(visible = visible, enter = enter, exit = exit) {
                render(item.first)

                DisposableEffect(item) {
                    onDispose {
                        if (!item.second.value) {
                            renderedItems.remove(item)
                        }
                    }
                }
            }
        }
    }
}
