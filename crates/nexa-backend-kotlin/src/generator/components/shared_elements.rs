use nexa_codegen::SourceWriter;

use crate::generator::{engine::features::Features, engine::imports::ImportSet};

pub(crate) fn imports(features: &Features, imports: &mut ImportSet) {
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.animation.AnimatedVisibility",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.animation.AnimatedVisibilityScope",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.animation.ExperimentalSharedTransitionApi",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.animation.SharedTransitionLayout",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.animation.SharedTransitionScope",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.runtime.CompositionLocalProvider",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.runtime.staticCompositionLocalOf",
    );
    imports.add(
        features.uses_shared_elements,
        "androidx.compose.ui.Modifier",
    );
}

pub(crate) fn render(out: &mut SourceWriter) {
    out.push_str(
        r#"@OptIn(ExperimentalSharedTransitionApi::class)
internal val LocalNexaSharedTransitionScope = staticCompositionLocalOf<SharedTransitionScope?> { null }

@OptIn(ExperimentalSharedTransitionApi::class)
internal val LocalNexaAnimatedVisibilityScope = staticCompositionLocalOf<AnimatedVisibilityScope?> { null }

@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
internal fun NexaSharedTransitionContent(content: @Composable () -> Unit) {
    SharedTransitionLayout {
        val sharedScope = this@SharedTransitionLayout
        AnimatedVisibility(visible = true) {
            val visibilityScope = this@AnimatedVisibility
            CompositionLocalProvider(
                LocalNexaSharedTransitionScope provides sharedScope,
                LocalNexaAnimatedVisibilityScope provides visibilityScope,
            ) {
                content()
            }
        }
    }
}

@OptIn(ExperimentalSharedTransitionApi::class)
@Composable
internal fun nexaSharedElementModifier(key: Any): Modifier {
    val sharedScope = LocalNexaSharedTransitionScope.current ?: return Modifier
    val visibilityScope = LocalNexaAnimatedVisibilityScope.current ?: return Modifier
    return with(sharedScope) {
        Modifier.sharedElement(
            sharedContentState = rememberSharedContentState(key = key),
            animatedVisibilityScope = visibilityScope,
        )
    }
}

"#,
    );
}
