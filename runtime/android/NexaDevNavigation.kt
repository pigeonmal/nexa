package __NEXA_PACKAGE__

import android.util.Base64
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.core.tween
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.compose.ui.Modifier
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

internal val LocalNexaDevNavController = staticCompositionLocalOf<NavHostController?> { null }

internal fun resolveNavigatorIdFromScope(scope: String): String {
    if (scope.startsWith("navigator/")) {
        val afterPrefix = scope.removePrefix("navigator/")
        val entryIndex = afterPrefix.indexOf("/entry/")
        return if (entryIndex != -1) afterPrefix.substring(0, entryIndex) else afterPrefix
    }
    return "app/root"
}

internal fun NexaDevStateStore.encodeScreenRoute(
    screen: JSONObject,
    expressions: JSONArray,
    locals: Map<String, Any>,
    scope: String,
    navigatorId: String? = null,
    entryId: String? = null,
): String {
    val parameters = screen.optJSONArray("parameters") ?: JSONArray()
    val values = JSONObject()
    for (index in 0 until minOf(parameters.length(), expressions.length())) {
        val parameter = parameters.optJSONObject(index) ?: continue
        val name = parameter.optString("name")
        if (name.isNotEmpty()) values.put(name, evaluate(expressions.opt(index), locals, scope))
    }
    val navId = navigatorId
        ?: (locals["__nexa_navigation_stack_identity"] as? String)
        ?: resolveNavigatorIdFromScope(scope)
    return encodeScreenRoute(
        screen.optString("name"),
        values,
        entryId = entryId,
        navigatorId = navId,
    )
}

internal fun NexaDevStateStore.encodeScreenRoute(
    screen: String,
    values: JSONObject = JSONObject(),
    entryId: String? = null,
    navigatorId: String? = null,
): String {
    val id = entryId ?: ("entry_" + java.util.UUID.randomUUID().toString().take(8))
    val payload = JSONObject()
        .put("screen", screen)
        .put("parameters", values)
        .put("entryId", id)
    if (navigatorId != null) {
        payload.put("navigatorId", navigatorId)
    }
    val encoded = Base64.encodeToString(
        payload.toString().toByteArray(Charsets.UTF_8),
        Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING,
    )
    return "nexa/$encoded"
}

internal fun NexaDevStateStore.decodeScreenRoute(encoded: String): JSONObject? = runCatching {
    JSONObject(Base64.decode(encoded, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING).toString(Charsets.UTF_8))
}.getOrNull()

internal fun NexaDevStateStore.screenParameters(screen: JSONObject, values: JSONObject): Map<String, Any> {
    val parameters = screen.optJSONArray("parameters") ?: JSONArray()
    return buildMap {
        for (index in 0 until parameters.length()) {
            val parameter = parameters.optJSONObject(index) ?: continue
            val name = parameter.optString("name")
            val value = values.opt(name)
            if (name.isNotEmpty() && value != null && value != JSONObject.NULL) put(name, value)
        }
    }
}

private fun nexaDevFindAppearanceMode(value: Any?): Any? = when (value) {
    is JSONArray -> {
        var result: Any? = null
        for (index in 0 until value.length()) {
            result = nexaDevFindAppearanceMode(value.opt(index))
            if (result != null) break
        }
        result
    }
    is JSONObject -> {
        if (value.has("Appearance")) {
            value.optJSONObject("Appearance")?.opt("mode") ?: "system"
        } else {
            var result: Any? = null
            val keys = value.keys()
            while (keys.hasNext() && result == null) {
                result = nexaDevFindAppearanceMode(value.opt(keys.next()))
            }
            result
        }
    }
    else -> null
}

@Composable
internal fun RenderNavigationStack(
    fields: JSONObject,
    module: JSONObject,
    store: NexaDevStateStore,
    locals: Map<String, Any>,
    scope: String,
    modifier: Modifier = Modifier,
) {
    val screens = module.optJSONArray("screens") ?: JSONArray()
    val showNavigationBar = fields.optBoolean("show_navigation_bar", true)
    val rootScreen = screens.optJSONObject(fields.optInt("root", -1)) ?: return
    val rootScreenName = rootScreen.optString("name")
    val stackIdentity = if (scope.isNotEmpty()) scope else "app/root/$rootScreenName"
    // NavHost uses this as its start destination. Keep the route and the identity
    // used to classify that destination together across recompositions.
    val (rootEntryId, route) = remember(stackIdentity, rootScreenName, store.navigationEpoch) {
        val entryId = "root_${java.util.UUID.randomUUID().toString().take(8)}"
        val encodedRoute = store.encodeScreenRoute(
            rootScreen,
            fields.optJSONArray("arguments") ?: JSONArray(),
            locals,
            scope,
            navigatorId = stackIdentity,
            entryId = entryId,
        )
        entryId to encodedRoute
    }
    // Classify the root by its stable back-stack entry, not mutable controller
    // state. During a push, previousBackStackEntry changes while the root entry
    // remains composed for its exit transition.
    val rootScope = "screen/${rootScreen.optString("name")}"
    val rootRouteData = route.removePrefix("nexa/")
    val rootRouteParameters =
        store.decodeScreenRoute(rootRouteData)?.optJSONObject("parameters") ?: JSONObject()
    val rootParameters = store.screenParameters(rootScreen, rootRouteParameters)
    val rootLocals = store.locals(rootScope, rootParameters)
    val appearanceExpression = nexaDevFindAppearanceMode(rootScreen.optJSONArray("body"))
    val appearanceMode = appearanceExpression?.let {
        store.stringify(store.evaluatePresented(it, rootLocals, rootScope))
    }
    androidx.compose.runtime.key(store.navigationEpoch) {
        val navController = rememberNavController()
        val currentBackStackEntry by navController.currentBackStackEntryAsState()
        val canPop = currentBackStackEntry != null && navController.previousBackStackEntry != null
        val activeRouteData = currentBackStackEntry?.arguments?.getString("routeData")
        val activeScreenName = activeRouteData
            ?.let(store::decodeScreenRoute)
            ?.optString("screen")
        val activeScreen = activeScreenName?.let { name ->
            (0 until screens.length())
                .mapNotNull(screens::optJSONObject)
                .firstOrNull { it.optString("name") == name }
        } ?: rootScreen
        val activeStatusBar = activeScreen.optJSONObject("status_bar")
            ?: module.optJSONObject("status_bar")
        androidx.compose.runtime.DisposableEffect(navController, stackIdentity) {
            store.registerNavController(stackIdentity, navController)
            onDispose {
                store.unregisterNavController(stackIdentity, navController)
            }
        }
        BackHandler(enabled = canPop) {
            store.popNavigation(explicitController = navController, navigatorId = stackIdentity)
        }
        val context = androidx.compose.ui.platform.LocalContext.current
        var activityContext: android.content.Context = context
        while (activityContext is android.content.ContextWrapper && activityContext !is android.app.Activity) {
            activityContext = activityContext.baseContext
        }
        val activity = activityContext as? android.app.Activity
        var activeDeepLinkUri by androidx.compose.runtime.remember {
            androidx.compose.runtime.mutableStateOf(activity?.intent?.data)
        }
        androidx.compose.runtime.DisposableEffect(activity) {
            val componentActivity = activity as? androidx.activity.ComponentActivity
            val listener = androidx.core.util.Consumer<android.content.Intent> { intent ->
                activeDeepLinkUri = intent.data
            }
            componentActivity?.addOnNewIntentListener(listener)
            onDispose {
                componentActivity?.removeOnNewIntentListener(listener)
            }
        }
        androidx.compose.runtime.LaunchedEffect(activeDeepLinkUri) {
            val uri = activeDeepLinkUri ?: return@LaunchedEffect
            val scheme = uri.scheme?.lowercase() ?: ""
            val segments = buildList {
                if (scheme !in listOf("http", "https")) uri.host?.let(::add)
                addAll(uri.pathSegments)
            }
            if (segments.isEmpty()) return@LaunchedEffect
            val routeSlug = segments[0].lowercase()
            for (screenIdx in 0 until screens.length()) {
                val sc = screens.optJSONObject(screenIdx) ?: continue
                val name = sc.optString("name")
                val slug = name.replace(Regex("([a-z])([A-Z])"), "$1-$2").lowercase()
                if (slug != routeSlug && !name.equals(routeSlug, ignoreCase = true)) continue
                val parameters = sc.optJSONArray("parameters") ?: org.json.JSONArray()
                if (segments.size > parameters.length() + 1) continue
                val values = JSONObject()
                var valid = true
                for (paramIdx in 0 until parameters.length()) {
                    val param = parameters.optJSONObject(paramIdx) ?: continue
                    val paramName = param.optString("name")
                    val raw = segments.getOrNull(paramIdx + 1) ?: uri.getQueryParameter(paramName)
                    if (raw == null) {
                        valid = false
                        break
                    }
                    val paramType = param.optString("type")
                    val parsed: Any? = when (paramType) {
                        "Bool" -> raw.toBooleanStrictOrNull()
                        "Int32", "Int", "Int64" -> raw.toIntOrNull()
                        "Float32", "Float64", "Double" -> raw.toDoubleOrNull()
                        else -> raw
                    }
                    if (parsed == null) {
                        valid = false
                        break
                    }
                    values.put(paramName, parsed)
                }
                if (!valid) continue
                val targetRoute = store.encodeScreenRoute(name, values, navigatorId = stackIdentity)
                navController.navigate(targetRoute)
                break
            }
        }
        NexaDevStatusBarContent(activeStatusBar, isSystemInDarkTheme()) {
            NavHost(
                navController = navController,
                startDestination = route,
                modifier = modifier.fillMaxSize(),
                enterTransition = {
                    slideIntoContainer(
                        towards = AnimatedContentTransitionScope.SlideDirection.Start,
                        animationSpec = tween(durationMillis = 350),
                    )
                },
                exitTransition = {
                    slideOutOfContainer(
                        towards = AnimatedContentTransitionScope.SlideDirection.Start,
                        animationSpec = tween(durationMillis = 350),
                    )
                },
                popEnterTransition = {
                    slideIntoContainer(
                        towards = AnimatedContentTransitionScope.SlideDirection.End,
                        animationSpec = tween(durationMillis = 350),
                    )
                },
                popExitTransition = {
                    slideOutOfContainer(
                        towards = AnimatedContentTransitionScope.SlideDirection.End,
                        animationSpec = tween(durationMillis = 350),
                    )
                },
            ) {
            composable("nexa/{routeData}") { backStackEntry ->
                val routeData = backStackEntry.arguments?.getString("routeData")
                val destination = routeData?.let(store::decodeScreenRoute)
                val screenName = destination?.optString("screen")
                // The graph keeps destination lambdas alive across module patches.
                // Read the current snapshot here so only the body definition changes;
                // the back-stack entry and mounted native component identity survive.
                val activeModule = store.module ?: module
                val activeScreens = activeModule.optJSONArray("screens") ?: JSONArray()
                val screen = (0 until activeScreens.length())
                    .mapNotNull(activeScreens::optJSONObject)
                    .firstOrNull { it.optString("name") == screenName }
                if (screen != null && destination != null) {
                    val animatedVisibilityScope = this
                    val parameters = store.screenParameters(
                        screen,
                        destination.optJSONObject("parameters") ?: JSONObject(),
                    )
                    val destinationNavId = destination.optString("navigatorId").takeIf { it.isNotEmpty() }
                    val navigatorId = destinationNavId ?: stackIdentity
                    val entryId = destination.optString("entryId").ifEmpty { backStackEntry.id }
                    val isRootScreen = screen.optString("name") == rootScreenName && entryId == rootEntryId
                    val screenScope = if (isRootScreen) {
                        "screen/${screen.optString("name")}"
                    } else {
                        "navigator/$navigatorId/entry/$entryId/screen/${screen.optString("name")}"
                    }
                    val screenNodes = screen.optJSONArray("body") ?: JSONArray()
                    val screenToolbars = nexaDevTabToolbars(screenNodes)
                    val screenBody = if (isRootScreen && (!showNavigationBar || screenToolbars.isEmpty())) {
                        screenNodes
                    } else {
                        nexaDevWithoutTabToolbars(screenNodes)
                    }
                    val screenBindings = store.bindings(
                        screen.optJSONArray("states"),
                        parameters,
                        screenScope,
                    ).toMutableMap()
                    screenBindings["__nexa_navigation_controller"] = navController
                    screenBindings["__nexa_navigation_stack_identity"] = navigatorId
                    screenBindings["__nexa_navigation_entry_id"] = entryId
                    CompositionLocalProvider(
                        LocalNexaDevNavController provides navController,
                        LocalNexaAnimatedVisibilityScope provides animatedVisibilityScope,
                    ) {
                        val screenContent: @Composable () -> Unit = {
                            NexaDevScreenLifecycle(screen, store, screenBindings, screenScope)
                            NexaDevNodeList(
                                screenBody,
                                activeModule,
                                store,
                                screenBindings,
                                screenScope,
                                modifier = Modifier.fillMaxSize(),
                            )
                        }
                        val routeContent: @Composable () -> Unit = {
                            if (!showNavigationBar || (isRootScreen && screenToolbars.isEmpty())) {
                                screenContent()
                            } else {
                                NexaNavigationScreenPrimitive(
                                    title = screen.optString("name"),
                                    inline = fields.optString("title_mode", "Inline") == "Inline",
                                    leadingTitle = when (fields.optString("title_alignment", "Automatic")) {
                                        "Leading" -> true
                                        else -> false
                                    },
                                    showBackButton = !isRootScreen,
                                    hasTrailingActions = screenToolbars.any {
                                        it.first == "Trailing" && it.second.length() > 0
                                    },
                                    onBack = {
                                        store.popNavigation(explicitController = navController, navigatorId = stackIdentity)
                                    },
                                    leadingActions = {
                                        RenderTabToolbarItems(
                                            screenToolbars,
                                            "Leading",
                                            activeModule,
                                            store,
                                            screenBindings,
                                            screenScope,
                                        )
                                    },
                                    actions = {
                                        RenderTabToolbarItems(
                                            screenToolbars,
                                            "Trailing",
                                            activeModule,
                                            store,
                                            screenBindings,
                                            screenScope,
                                        )
                                    },
                                    content = screenContent,
                                )
                            }
                        }
                        if (appearanceMode == null) {
                            routeContent()
                        } else {
                            NexaAppearancePrimitive(mode = appearanceMode) {
                                routeContent()
                            }
                        }
                    }
                }
            }
        }
        }
    }
}

@Composable
internal fun NexaDevScreenLifecycle(
    screen: JSONObject,
    store: NexaDevStateStore,
    parameters: Map<String, Any>,
    scope: String = "screen/${screen.optString("name")}",
) {
    val lifecycleOwner = LocalLifecycleOwner.current
    val latestScreen = rememberUpdatedState(screen)
    val navigationController = LocalNexaDevNavController.current
    val actionParameters = if (navigationController == null) parameters else {
        parameters + ("__nexa_navigation_controller" to navigationController)
    }
    val latestParameters = rememberUpdatedState(actionParameters)
    val coroutineScope = rememberCoroutineScope()
    DisposableEffect(lifecycleOwner, scope) {
        val observer = LifecycleEventObserver { _, event ->
            val currentScreen = latestScreen.value
            val actions = when (event) {
                Lifecycle.Event.ON_RESUME -> currentScreen.optJSONArray("on_appear")
                Lifecycle.Event.ON_PAUSE -> currentScreen.optJSONArray("on_disappear")
                else -> null
            }
            if (event == Lifecycle.Event.ON_RESUME) {
                store.activeScreenParameters[scope] = latestParameters.value
            }
            actions?.let {
                if (event == Lifecycle.Event.ON_RESUME && currentScreen.optBoolean("on_appear_async")) {
                    coroutineScope.launch { store.performAsync(it, scope, latestParameters.value) }
                } else {
                    store.perform(it, scope, latestParameters.value)
                }
            }
            if (event == Lifecycle.Event.ON_PAUSE) {
                store.clearNativeEventSubscriptions(scope)
                store.clearNativeTasks(scope)
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            store.clearScope(scope)
        }
    }
}
