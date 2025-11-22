package app.musicopy.ui.components

import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.style.TextOverflow
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.arrow_back_24px
import musicopy_root.musicopy.generated.resources.more_vert_24px
import org.jetbrains.compose.resources.painterResource

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TopBar(
    title: String,
    onShowNodeStatus: () -> Unit,
    onBack: (() -> Unit)? = null,
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

    val nodeInfoButton = @Composable {
        IconButton(onClick = onShowNodeStatus) {
            Icon(
                painter = painterResource(Res.drawable.more_vert_24px),
                contentDescription = "Node info"
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
            actions = {
                nodeInfoButton();
            },
        )
    } else {
        TopAppBar(
            colors = colors,
            title = title,
            actions = {
                // TODO
                nodeInfoButton();
            },
        )
    }
}