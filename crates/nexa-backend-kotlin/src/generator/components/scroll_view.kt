// nexa-codegen: NexaScrollViewPrimitive
@androidx.compose.runtime.Composable
internal fun NexaScrollViewPrimitive(
    axis: String,
    modifier: androidx.compose.ui.Modifier = androidx.compose.ui.Modifier,
    componentRef: NexaScrollViewRef? = null,
    content: @androidx.compose.runtime.Composable () -> Unit,
) {
    val scrollState = androidx.compose.foundation.rememberScrollState()
    val coroutineScope = androidx.compose.runtime.rememberCoroutineScope()
    val density = androidx.compose.ui.platform.LocalDensity.current
    val vertical = axis == "Vertical"

    androidx.compose.runtime.DisposableEffect(componentRef, scrollState, coroutineScope, density, vertical) {
        val generation = componentRef?.attach(
            scrollToTop = { animated ->
                coroutineScope.launch {
                    if (vertical) {
                        if (animated) scrollState.animateScrollTo(0) else scrollState.scrollTo(0)
                    } else {
                        if (animated) scrollState.animateScrollTo(0) else scrollState.scrollTo(0)
                    }
                }
            },
            scrollToEnd = { animated ->
                coroutineScope.launch {
                    val end = scrollState.maxValue
                    if (animated) scrollState.animateScrollTo(end) else scrollState.scrollTo(end)
                }
            },
            scrollToOffset = { offset, animated ->
                coroutineScope.launch {
                    val requested = with(density) { offset.coerceAtLeast(0.0).dp.roundToPx() }
                    val target = requested.coerceIn(0, scrollState.maxValue)
                    if (animated) scrollState.animateScrollTo(target) else scrollState.scrollTo(target)
                }
            },
        )
        onDispose {
            if (generation != null) componentRef?.detach(generation)
        }
    }

    if (vertical) {
        androidx.compose.foundation.layout.Column(
            modifier = modifier.verticalScroll(scrollState),
        ) {
            NexaScrollContentPrimitive(LocalNexaScrollContentPadding.current) { padding ->
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier.padding(padding),
                ) { content() }
            }
        }
    } else {
        androidx.compose.foundation.layout.Row(
            modifier = modifier.horizontalScroll(scrollState),
        ) {
            NexaScrollContentPrimitive(LocalNexaScrollContentPadding.current) { padding ->
                androidx.compose.foundation.layout.Box(
                    modifier = androidx.compose.ui.Modifier.padding(padding),
                ) { content() }
            }
        }
    }
}
