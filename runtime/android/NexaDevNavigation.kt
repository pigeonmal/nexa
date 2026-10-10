package __NEXA_PACKAGE__

import android.util.Base64
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

internal val LocalNexaDevNavController = staticCompositionLocalOf<NavHostController?> { null }

internal fun NexaDevStateStore.encodeScreenRoute(
    screen: JSONObject,
    expressions: JSONArray,
    locals: Map<String, Any>,
    scope: String,
): String {
    val parameters = screen.optJSONArray("parameters") ?: JSONArray()
    val values = JSONObject()
    for (index in 0 until minOf(parameters.length(), expressions.length())) {
        val parameter = parameters.optJSONObject(index) ?: continue
        val name = parameter.optString("name")
        if (name.isNotEmpty()) values.put(name, evaluate(expressions.opt(index), locals, scope))
    }
    return encodeScreenRoute(screen.optString("name"), values)
}

internal fun NexaDevStateStore.encodeScreenRoute(screen: String, values: JSONObject = JSONObject()): String {
    val payload = JSONObject().put("screen", screen).put("parameters", values)
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
) {
    val screens = module.optJSONArray("screens") ?: JSONArray()
    val rootScreen = screens.optJSONObject(fields.optInt("root", -1)) ?: return
    val route = store.encodeScreenRoute(
        rootScreen,
        fields.optJSONArray("arguments") ?: JSONArray(),
        locals,
        scope,
    )
    val rootScope = "screen/${rootScreen.optString("name")}"
    val rootParameters = store.screenParameters(
        rootScreen,
        store.decodeScreenRoute(route)?.optJSONObject("parameters") ?: JSONObject(),
    )
    val rootLocals = store.locals(rootScope, rootParameters)
    val appearanceExpression = nexaDevFindAppearanceMode(rootScreen.optJSONArray("body"))
    val appearanceMode = appearanceExpression?.let {
        store.stringify(store.evaluatePresented(it, rootLocals, rootScope))
    }
    androidx.compose.runtime.key(store.navigationEpoch) {
        val navController = rememberNavController()
        NavHost(navController = navController, startDestination = route) {
            composable("nexa/{routeData}") { backStackEntry ->
                val routeData = backStackEntry.arguments?.getString("routeData")
                val destination = routeData?.let(store::decodeScreenRoute)
                val screenName = destination?.optString("screen")
                val screen = (0 until screens.length())
                    .mapNotNull(screens::optJSONObject)
                    .firstOrNull { it.optString("name") == screenName }
                if (screen != null && destination != null) {
                    val animatedVisibilityScope = this
                    val parameters = store.screenParameters(
                        screen,
                        destination.optJSONObject("parameters") ?: JSONObject(),
                    )
                    CompositionLocalProvider(
                        LocalNexaDevNavController provides navController,
                        LocalNexaAnimatedVisibilityScope provides animatedVisibilityScope,
                    ) {
                        val screenContent: @Composable () -> Unit = {
                            NexaDevScreenLifecycle(screen, store, parameters)
                            NexaDevNodeList(
                                screen.optJSONArray("body") ?: JSONArray(),
                                module,
                                store,
                                parameters,
                                "screen/${screen.optString("name")}",
                            )
                        }
                        val routeContent: @Composable () -> Unit = {
                            if (screen.optString("name") == rootScreen.optString("name")) {
                                screenContent()
                            } else {
                                NexaNavigationScreenPrimitive(
                                    title = screen.optString("name"),
                                    onBack = { navController.popBackStack() },
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

@Composable
internal fun NexaDevScreenLifecycle(
    screen: JSONObject,
    store: NexaDevStateStore,
    parameters: Map<String, Any>,
) {
    val lifecycleOwner = LocalLifecycleOwner.current
    val latestScreen = rememberUpdatedState(screen)
    val latestParameters = rememberUpdatedState(parameters)
    val coroutineScope = rememberCoroutineScope()
    val scope = "screen/${screen.optString("name")}"
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
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
}
