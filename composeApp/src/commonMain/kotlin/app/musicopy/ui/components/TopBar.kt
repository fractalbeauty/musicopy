package app.musicopy.ui.components

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.style.TextOverflow
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_back_24px
import musicopy_root.musicopy.generated.resources.more_vert_24px
import musicopy_root.musicopy.generated.resources.network_node_24px
import org.jetbrains.compose.resources.DrawableResource
import org.jetbrains.compose.resources.painterResource

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopBar(
    title: String,
    onShowNodeStatus: () -> Unit,
    onBack: (() -> Unit)? = null,
    extraActions: @Composable () -> Unit = {},
    extraMenuItems: @Composable (onDismiss: () -> Unit) -> Unit = {},
) {
    val colors = TopAppBarDefaults.topAppBarColors(
        containerColor = MaterialTheme.colorScheme.primaryContainer,
        titleContentColor = MaterialTheme.colorScheme.primary,
    )

    val title = @Composable {
        Text(
            title,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis
        )
    }

    val actions = @Composable {
        extraActions()

        var expanded by remember { mutableStateOf(false) }

        val onDismiss = {
            expanded = false
        }

        IconButton(onClick = { expanded = !expanded }) {
            Icon(
                painter = painterResource(Res.drawable.more_vert_24px),
                contentDescription = "More options"
            )
        }
        DropdownMenu(
            expanded = expanded,
            onDismissRequest = onDismiss
        ) {
            extraMenuItems(onDismiss)

            DropdownMenuItem(
                leadingIcon = {
                    Icon(
                        painter = painterResource(Res.drawable.network_node_24px),
                        contentDescription = null,
                    )
                },
                text = {
                    Text("Network stats", style = MaterialTheme.typography.labelLarge)
                },
                onClick = {
                    onDismiss()
                    onShowNodeStatus()
                }
            )
        }
    }

    if (onBack !== null) {
        TopAppBar(
            colors = colors,
            title = title,
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(
                        painter = painterResource(Res.drawable.arrow_back_24px),
                        contentDescription = "Back"
                    )
                }
            },
            actions = { actions() },
        )
    } else {
        TopAppBar(
            colors = colors,
            title = title,
            actions = { actions() },
        )
    }
}

@Composable
fun TopBarMenuItem(
    icon: DrawableResource,
    text: String,
    onClick: () -> Unit,
) {
    DropdownMenuItem(
        leadingIcon = {
            Icon(
                painter = painterResource(icon),
                contentDescription = null,
            )
        },
        text = {
            Text(text, style = MaterialTheme.typography.labelLarge)
        },
        onClick = onClick
    )
}
