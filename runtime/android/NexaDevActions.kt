package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import java.lang.ref.WeakReference

internal enum class NexaDevActionFlow { Normal, Break, Continue }

internal sealed interface NexaDevFunctionFlow {
    object Normal : NexaDevFunctionFlow
    object Break : NexaDevFunctionFlow
    object Continue : NexaDevFunctionFlow
    data class Returned(val value: Any) : NexaDevFunctionFlow
}

internal fun Any?.orNullValue(): Any = this ?: JSONObject.NULL

internal fun NexaDevStateStore.perform(actions: JSONArray, scope: String, locals: Map<String, Any>) {
    performActions(actions, scope, locals)
}

internal fun NexaDevStateStore.performActions(actions: JSONArray, scope: String, locals: Map<String, Any>): NexaDevActionFlow {
    for (index in 0 until actions.length()) {
        if (pendingPluginFailure != null) return NexaDevActionFlow.Normal
        val raw = actions.opt(index)
        if (raw is String) {
            if (raw == "Break") return NexaDevActionFlow.Break
            if (raw == "Continue") return NexaDevActionFlow.Continue
            continue
        }
        val action = raw as? JSONObject ?: continue
        val assign = action.optJSONObject("Assign")
        if (assign != null) {
            val name = assign.optString("name")
            val nextValue = evaluate(assign.opt("value"), locals, scope)
            setState(name, nextValue, scope)
            moduleRevision++
            continue
        }
        val expression = action.opt("Expression")
        if (expression != null) {
            evaluate(expression, locals, scope)
            continue
        }
        val branch = action.optJSONObject("If")
        if (branch != null) {
            val selected = if (truthy(evaluate(branch.opt("condition"), locals, scope))) {
                branch.optJSONArray("then_branch")
            } else {
                branch.optJSONArray("else_branch")
            }
            val flow = performActions(selected ?: JSONArray(), scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            continue
        }
        val loop = action.optJSONObject("For")
        if (loop != null) {
            val name = loop.optString("name")
            val iterable = evaluate(loop.opt("iterable"), locals, scope)
            val items = when (iterable) {
                is List<*> -> iterable
                is Set<*> -> iterable.toList()
                else -> emptyList<Any?>()
            }
            iteration@ for (item in items) {
                val nextLocals = locals.toMutableMap()
                nextLocals[name] = item.orNullValue()
                when (performActions(loop.optJSONArray("body") ?: JSONArray(), scope, nextLocals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val mapLoop = action.optJSONObject("ForMap")
        if (mapLoop != null) {
            val keyName = mapLoop.optString("key_name")
            val valueName = mapLoop.optString("value_name")
            val iterable = evaluate(mapLoop.opt("iterable"), locals, scope)
            val map = iterable as? Map<*, *> ?: emptyMap<Any?, Any?>()
            iteration@ for ((key, value) in map) {
                val nextLocals = locals.toMutableMap()
                nextLocals[keyName] = key.orNullValue()
                nextLocals[valueName] = value.orNullValue()
                when (performActions(mapLoop.optJSONArray("body") ?: JSONArray(), scope, nextLocals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val whileLoop = action.optJSONObject("While")
        if (whileLoop != null) {
            var iterations = 0
            iteration@ while (iterations++ < 10_000 && truthy(evaluate(whileLoop.opt("condition"), locals, scope))) {
                when (performActions(whileLoop.optJSONArray("body") ?: JSONArray(), scope, locals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val nativeAssignment = action.optJSONObject("NativePropertyAssign")
        if (nativeAssignment != null) {
            val receiver = evaluate(nativeAssignment.opt("receiver"), locals, scope)
            val property = nativeAssignment.optString("property")
            val value = evaluate(nativeAssignment.opt("value"), locals, scope)
            if (!NexaDevPluginBridge.writeInstanceProperty(receiver, property, value)) {
                android.util.Log.w("NexaDevRuntime", "Native property write is unsupported: $property")
            }
            continue
        }
        val subscription = action.optJSONObject("NativeEventSubscribe")
        if (subscription != null) {
            val receiver = evaluate(subscription.opt("receiver"), locals, scope)
            val property = subscription.optString("property")
            if (!subscribeNativeEvent(
                receiver = receiver,
                property = property,
                actions = subscription.optJSONArray("actions") ?: JSONArray(),
                parameterNames = subscription.optJSONArray("parameters") ?: JSONArray(),
                scope = scope,
                locals = locals,
            )) {
                android.util.Log.w("NexaDevRuntime", "Native event subscription is unsupported: $property")
            }
            continue
        }
        val networkSubscription = action.optJSONObject("NetworkStatusSubscribe")
        if (networkSubscription != null) {
            val parameter = networkSubscription.optString("parameter")
            if (parameter.isNotEmpty()) {
                subscribeNetworkStatus(
                    parameter = parameter,
                    actions = networkSubscription.optJSONArray("actions") ?: JSONArray(),
                    scope = scope,
                    locals = locals,
                )
            }
            continue
        }
        val taskLaunch = action.optJSONObject("TaskLaunch")
        if (taskLaunch != null) {
            launchNativeTask(
                handle = taskLaunch.optString("handle").takeIf { taskLaunch.has("handle") && !taskLaunch.isNull("handle") },
                executor = taskLaunch.optString("executor"),
                actions = taskLaunch.optJSONArray("actions") ?: JSONArray(),
                scope = scope,
                locals = locals,
            )
            continue
        }
        val taskCancel = action.optJSONObject("TaskCancel")
        if (taskCancel != null) {
            cancelNativeTask(scope, taskCancel.optString("handle"))
            continue
        }
        val withAnimation = action.optJSONObject("WithAnimation")
        if (withAnimation != null) {
            val flow = performWithAnimation(withAnimation, scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            continue
        }
        val mutation = action.optJSONObject("CollectionMutation")
        if (mutation != null) {
            mutateCollection(mutation, scope, locals)
            continue
        }
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            pendingPluginFailure = null
            val flow = performActions(tryCatch.optJSONArray("body") ?: JSONArray(), scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            val failure = pendingPluginFailure
            pendingPluginFailure = null
            if (failure != null) {
                val caught = performPluginFailureCatch(
                    failure,
                    tryCatch.optJSONArray("error_catches") ?: JSONArray(),
                    scope,
                    locals,
                )
                if (caught != null) {
                    if (caught != NexaDevActionFlow.Normal) return caught
                } else {
                    val catchBody = tryCatch.optJSONArray("catch_body")
                    if (catchBody != null) {
                        val catchFlow = performActions(catchBody, scope, locals)
                        if (catchFlow != NexaDevActionFlow.Normal) return catchFlow
                    } else {
                        pendingPluginFailure = failure
                    }
                }
            }
            continue
        }
        if (action.has("Break")) return NexaDevActionFlow.Break
        if (action.has("Continue")) return NexaDevActionFlow.Continue
    }
    return NexaDevActionFlow.Normal
}

internal fun NexaDevStateStore.performFunction(actions: JSONArray, scope: String, locals: Map<String, Any>): NexaDevFunctionFlow {
    val functionLocals = locals.toMutableMap()
    for (index in 0 until actions.length()) {
        val action = actions.optJSONObject(index) ?: continue
        val binding = action.optJSONObject("Let")
        if (binding != null) {
            functionLocals[binding.optString("name")] = evaluate(binding.opt("value"), functionLocals, scope)
            continue
        }
        val returned = action.optJSONObject("Return")
        if (returned != null) return NexaDevFunctionFlow.Returned(evaluate(returned.opt("value"), functionLocals, scope))
        val assignment = action.optJSONObject("Assign")
        if (assignment != null) {
            setState(assignment.optString("name"), evaluate(assignment.opt("value"), functionLocals, scope), scope)
            moduleRevision++
            continue
        }
        val expression = action.opt("Expression")
        if (expression != null) {
            evaluate(expression, functionLocals, scope)
            if (pendingPluginFailure != null) return NexaDevFunctionFlow.Normal
            continue
        }
        val branch = action.optJSONObject("If")
        if (branch != null) {
            val selected = if (truthy(evaluate(branch.opt("condition"), functionLocals, scope))) {
                branch.optJSONArray("then_branch")
            } else branch.optJSONArray("else_branch")
            when (val flow = performFunction(selected ?: JSONArray(), scope, functionLocals)) {
                NexaDevFunctionFlow.Normal -> Unit
                else -> return flow
            }
            continue
        }
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            pendingPluginFailure = null
            val flow = performFunction(tryCatch.optJSONArray("body") ?: JSONArray(), scope, functionLocals)
            val failure = pendingPluginFailure
            pendingPluginFailure = null
            if (failure != null) {
                val caught = performFunctionPluginFailureCatch(
                    failure,
                    tryCatch.optJSONArray("error_catches") ?: JSONArray(),
                    scope,
                    functionLocals,
                )
                if (caught != null) {
                    when (caught) {
                        NexaDevFunctionFlow.Normal -> Unit
                        else -> return caught
                    }
                } else {
                    val catchBody = tryCatch.optJSONArray("catch_body")
                    if (catchBody != null) {
                        when (val catchFlow = performFunction(catchBody, scope, functionLocals)) {
                            NexaDevFunctionFlow.Normal -> Unit
                            else -> return catchFlow
                        }
                    } else pendingPluginFailure = failure
                }
            } else when (flow) {
                NexaDevFunctionFlow.Normal -> Unit
                else -> return flow
            }
            continue
        }
        val loop = action.optJSONObject("For")
        if (loop != null) {
            val iterable = evaluate(loop.opt("iterable"), functionLocals, scope)
            val items = when (iterable) {
                is List<*> -> iterable
                is Set<*> -> iterable.toList()
                else -> emptyList<Any?>()
            }
            for (item in items) {
                val iterationLocals = functionLocals.toMutableMap()
                iterationLocals[loop.optString("name")] = item.orNullValue()
                when (val flow = performFunction(loop.optJSONArray("body") ?: JSONArray(), scope, iterationLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        val mapLoop = action.optJSONObject("ForMap")
        if (mapLoop != null) {
            val iterable = evaluate(mapLoop.opt("iterable"), functionLocals, scope) as? Map<*, *> ?: emptyMap<Any?, Any?>()
            for ((key, value) in iterable) {
                val iterationLocals = functionLocals.toMutableMap()
                iterationLocals[mapLoop.optString("key_name")] = key.orNullValue()
                iterationLocals[mapLoop.optString("value_name")] = value.orNullValue()
                when (val flow = performFunction(mapLoop.optJSONArray("body") ?: JSONArray(), scope, iterationLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        val whileLoop = action.optJSONObject("While")
        if (whileLoop != null) {
            var iterations = 0
            while (iterations++ < 10_000 && truthy(evaluate(whileLoop.opt("condition"), functionLocals, scope))) {
                when (val flow = performFunction(whileLoop.optJSONArray("body") ?: JSONArray(), scope, functionLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        if (action.has("Break")) return NexaDevFunctionFlow.Break
        if (action.has("Continue")) return NexaDevFunctionFlow.Continue
    }
    return NexaDevFunctionFlow.Normal
}

private fun NexaDevStateStore.performFunctionPluginFailureCatch(
    failure: NexaDevPluginFailure,
    arms: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevFunctionFlow? {
    for (index in 0 until arms.length()) {
        val arm = arms.optJSONObject(index) ?: continue
        if (arm.optString("namespace") != failure.namespace ||
            arm.optString("error_type") != failure.errorType ||
            arm.optString("variant") != failure.variant
        ) continue
        val catchLocals = locals.toMutableMap()
        val parameters = arm.optJSONArray("parameters") ?: JSONArray()
        for (parameterIndex in 0 until parameters.length()) {
            val tuple = parameters.optJSONArray(parameterIndex) ?: continue
            if (tuple.length() < 2) continue
            catchLocals[tuple.optString(0)] = failure.payload[tuple.optString(1)] ?: JSONObject.NULL
        }
        return performFunction(arm.optJSONArray("body") ?: JSONArray(), scope, catchLocals)
    }
    return null
}

private fun NexaDevStateStore.launchNativeTask(
    handle: String?,
    executor: String,
    actions: JSONArray,
    scope: String,
    locals: Map<String, Any>,
) {
    val key = handle?.takeIf { it.isNotEmpty() }?.let { "$scope/task/$it" }
        ?: "$scope/task/fire-and-forget/${java.util.UUID.randomUUID()}"
    if (handle?.isNotEmpty() == true) foregroundTasks.remove(key)?.cancel()
    val dispatcher = if (executor == "Background") Dispatchers.Default else Dispatchers.Main.immediate
    val exceptionHandler = CoroutineExceptionHandler { _, error ->
        if (error !is CancellationException) {
            android.util.Log.e("NexaDevRuntime", "Foreground task failed", error)
        }
    }
    val job = eventScope.launch(context = dispatcher + exceptionHandler, start = CoroutineStart.LAZY) {
        try {
            performAsync(actions, scope, locals.toMap())
        } catch (cancellation: CancellationException) {
            throw cancellation
        } catch (error: Throwable) {
            android.util.Log.e("NexaDevRuntime", "Foreground task failed", error)
        }
    }
    foregroundTasks[key] = job
    job.start()
}

private fun NexaDevStateStore.cancelNativeTask(scope: String, handle: String) {
    if (handle.isEmpty()) return
    foregroundTasks.remove("$scope/task/$handle")?.cancel()
}

internal fun NexaDevStateStore.clearNativeTasks(scope: String? = null) {
    val prefix = scope?.let { "$it/task/" }
    val selected = foregroundTasks.keys.filter { prefix == null || it.startsWith(prefix) }
    for (key in selected) foregroundTasks.remove(key)?.cancel()
}

internal fun NexaDevStateStore.devNativeEventHandler(
    actions: JSONArray,
    parameterNames: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): (List<Any>) -> Unit {
    val storeReference = WeakReference(this)
    return { arguments ->
        storeReference.get()?.eventScope?.launch {
            val store = storeReference.get() ?: return@launch
            val eventLocals = locals.toMutableMap()
            for (index in 0 until minOf(parameterNames.length(), arguments.size)) {
                val name = parameterNames.optString(index)
                if (name.isNotEmpty()) eventLocals[name] = arguments[index]
            }
            try {
                store.performAsync(actions, scope, eventLocals)
            } catch (error: CancellationException) {
                throw error
            } catch (error: Throwable) {
                android.util.Log.e("NexaDevRuntime", "Native event action failed", error)
            }
        }
    }
}

internal fun NexaDevStateStore.subscribeNativeEvent(
    receiver: Any,
    property: String,
    actions: JSONArray,
    parameterNames: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): Boolean {
    val handler = devNativeEventHandler(actions, parameterNames, scope, locals)
    if (!NexaDevPluginBridge.subscribeInstanceEvent(receiver, property, handler)) return false
    nativeEventSubscriptions.removeAll { it.receiver === receiver && it.property == property }
    nativeEventSubscriptions += NexaDevNativeEventSubscription(
        receiver = receiver,
        property = property,
        actions = actions,
        parameters = parameterNames,
        scope = scope,
        locals = locals.toMap(),
    )
    return true
}

internal fun NexaDevStateStore.subscribeNetworkStatus(
    parameter: String,
    actions: JSONArray,
    scope: String,
    locals: Map<String, Any>,
) {
    val id = "$scope/$parameter"
    val parameters = JSONArray().put(parameter)
    val handler = devNativeEventHandler(actions, parameters, scope, locals)
    NexaDevNetworkStatus.subscribe(context, id) { online -> handler(listOf(online)) }
    networkStatusSubscriptions.removeAll { it.id == id }
    networkStatusSubscriptions += NexaDevNetworkStatusSubscription(
        id = id,
        parameter = parameter,
        actions = actions,
        scope = scope,
        locals = locals.toMap(),
    )
}

internal fun NexaDevStateStore.refreshNativeEventSubscriptions(module: JSONObject) {
    val previousNetwork = networkStatusSubscriptions.toList()
    networkStatusSubscriptions.clear()
    previousNetwork.forEach { NexaDevNetworkStatus.clear(it.id) }
    for (subscription in previousNetwork) {
        val replacement = networkStatusAction(module, subscription.scope, subscription.parameter) ?: continue
        subscribeNetworkStatus(
            parameter = subscription.parameter,
            actions = replacement.optJSONArray("actions") ?: JSONArray(),
            scope = subscription.scope,
            locals = subscription.locals,
        )
    }
    val previous = nativeEventSubscriptions.toList()
    nativeEventSubscriptions.clear()
    previous.forEach { NexaDevPluginBridge.clearInstanceEvent(it.receiver, it.property) }
    val candidates = mutableListOf<JSONObject>()
    collectNativeEventSubscriptions(module, candidates)
    for (subscription in previous) {
        var replacement: JSONObject? = null
        for (candidate in candidates) {
            if (candidate.optString("property") != subscription.property) continue
            val receiverExpression = candidate.opt("receiver")
            val resolved = stableNativeEventReceiver(receiverExpression, subscription.locals, subscription.scope)
            if (resolved === subscription.receiver) replacement = candidate
        }
        val next = replacement ?: continue
        subscribeNativeEvent(
            receiver = subscription.receiver,
            property = subscription.property,
            actions = next.optJSONArray("actions") ?: JSONArray(),
            parameterNames = next.optJSONArray("parameters") ?: JSONArray(),
            scope = subscription.scope,
            locals = subscription.locals,
        )
    }

    installLifecycleNativeEventSubscriptions(
        module.optJSONArray("on_appear") ?: JSONArray(),
        "app",
        emptyMap(),
    )
    val screens = module.optJSONArray("screens") ?: JSONArray()
    for ((scope, parameters) in activeScreenParameters.toMap()) {
        val screenName = scope.substringAfterLast('/')
        val screen = (0 until screens.length())
            .mapNotNull(screens::optJSONObject)
            .firstOrNull { it.optString("name") == screenName }
            ?: continue
        installLifecycleNativeEventSubscriptions(
            screen.optJSONArray("on_appear") ?: continue,
            scope,
            parameters,
        )
    }
}

private fun NexaDevStateStore.installLifecycleNativeEventSubscriptions(
    actions: JSONArray,
    scope: String,
    locals: Map<String, Any>,
) {
    for (index in 0 until actions.length()) {
        val raw = actions.opt(index) as? JSONObject ?: continue
        val kind = raw.keys().asSequence().firstOrNull() ?: continue
        val fields = raw.optJSONObject(kind) ?: continue
        when (kind) {
            "NetworkStatusSubscribe" -> {
                val parameter = fields.optString("parameter").takeIf(String::isNotEmpty) ?: continue
                val id = "$scope/$parameter"
                if (networkStatusSubscriptions.none { it.id == id }) {
                    subscribeNetworkStatus(
                        parameter = parameter,
                        actions = fields.optJSONArray("actions") ?: JSONArray(),
                        scope = scope,
                        locals = locals,
                    )
                }
            }
            "NativeEventSubscribe" -> {
                val receiverExpression = fields.opt("receiver") ?: continue
                val receiver = stableNativeEventReceiver(receiverExpression, locals, scope) ?: continue
                val property = fields.optString("property")
                if (nativeEventSubscriptions.any { it.receiver === receiver && it.property == property }) continue
                subscribeNativeEvent(
                    receiver = receiver,
                    property = property,
                    actions = fields.optJSONArray("actions") ?: JSONArray(),
                    parameterNames = fields.optJSONArray("parameters") ?: JSONArray(),
                    scope = scope,
                    locals = locals,
                )
            }
            "If" -> {
                val selected = if (truthy(evaluate(fields.opt("condition"), locals, scope))) {
                    fields.optJSONArray("then_branch")
                } else {
                    fields.optJSONArray("else_branch")
                }
                if (selected != null) installLifecycleNativeEventSubscriptions(selected, scope, locals)
            }
            "For" -> {
                val name = fields.optString("name")
                val items = when (val iterable = evaluate(fields.opt("iterable"), locals, scope)) {
                    is List<*> -> iterable
                    is Set<*> -> iterable.toList()
                    else -> emptyList()
                }
                for (item in items) {
                    val iterationLocals = locals + (name to (item ?: org.json.JSONObject.NULL))
                    installLifecycleNativeEventSubscriptions(
                        fields.optJSONArray("body") ?: JSONArray(), scope, iterationLocals,
                    )
                }
            }
            "ForMap" -> {
                val values = evaluate(fields.opt("iterable"), locals, scope) as? Map<*, *> ?: emptyMap<Any, Any>()
                for ((key, value) in values.entries.sortedBy { stringify(it.key ?: org.json.JSONObject.NULL) }) {
                    val iterationLocals = locals + mapOf(
                        fields.optString("key_name") to stringify(key ?: org.json.JSONObject.NULL),
                        fields.optString("value_name") to (value ?: org.json.JSONObject.NULL),
                    )
                    installLifecycleNativeEventSubscriptions(
                        fields.optJSONArray("body") ?: JSONArray(), scope, iterationLocals,
                    )
                }
            }
            "While" -> {
                var iterations = 0
                while (iterations++ < 10_000 && truthy(evaluate(fields.opt("condition"), locals, scope))) {
                    installLifecycleNativeEventSubscriptions(
                        fields.optJSONArray("body") ?: JSONArray(), scope, locals,
                    )
                }
            }
        }
    }
}

internal fun NexaDevStateStore.clearNativeEventSubscriptions(scope: String? = null) {
    if (scope != null) activeScreenParameters.remove(scope)
    val removedNetwork = networkStatusSubscriptions.filter { scope == null || it.scope == scope }
    networkStatusSubscriptions.removeAll { scope == null || it.scope == scope }
    removedNetwork.forEach { NexaDevNetworkStatus.clear(it.id) }
    val removed = nativeEventSubscriptions.filter { scope == null || it.scope == scope }
    nativeEventSubscriptions.removeAll { scope == null || it.scope == scope }
    removed.forEach { NexaDevPluginBridge.clearInstanceEvent(it.receiver, it.property) }
}

private fun NexaDevStateStore.stableNativeEventReceiver(
    expression: Any?,
    locals: Map<String, Any>,
    scope: String,
): Any? {
    val tagged = expression as? JSONObject ?: return null
    val iterator = tagged.keys()
    if (!iterator.hasNext()) return null
    val kind = iterator.next()
    return when (kind) {
        "State" -> evaluate(expression, locals, scope)
        "Member" -> {
            val member = tagged.optJSONObject("Member") ?: return null
            val base = member.opt("base")
            if (stableNativeEventReceiver(base, locals, scope) == null) null else evaluate(expression, locals, scope)
        }
        else -> null
    }
}

private fun collectNativeEventSubscriptions(value: Any?, result: MutableList<JSONObject>) {
    when (value) {
        is JSONArray -> for (index in 0 until value.length()) collectNativeEventSubscriptions(value.opt(index), result)
        is JSONObject -> {
            value.optJSONObject("NativeEventSubscribe")?.let(result::add)
            val keys = value.keys()
            while (keys.hasNext()) collectNativeEventSubscriptions(value.opt(keys.next()), result)
        }
    }
}

private fun networkStatusAction(module: JSONObject, scope: String, parameter: String): JSONObject? {
    val roots = when {
        scope == "app" -> listOf("body", "on_appear", "on_active", "on_inactive", "on_background")
            .mapNotNull(module::opt)
        scope.startsWith("screen/") -> {
            val screenName = scope.substringAfterLast('/')
            val screens = module.optJSONArray("screens") ?: JSONArray()
            val screen = (0 until screens.length())
                .mapNotNull(screens::optJSONObject)
                .firstOrNull { it.optString("name") == screenName }
            screen?.let { value ->
                listOf("body", "on_appear", "on_disappear").mapNotNull(value::opt)
            } ?: emptyList()
        }
        scope.startsWith("component/") -> {
            val componentName = scope.substringAfterLast('/')
            val components = module.optJSONArray("components") ?: JSONArray()
            val component = (0 until components.length())
                .mapNotNull(components::optJSONObject)
                .firstOrNull { it.optString("name") == componentName }
            listOfNotNull(component?.opt("body"))
        }
        else -> listOf(module)
    }
    return roots.firstNotNullOfOrNull { findNetworkStatusAction(it, parameter) }
}

private fun findNetworkStatusAction(value: Any?, parameter: String): JSONObject? = when (value) {
    is JSONArray -> (0 until value.length())
        .firstNotNullOfOrNull { findNetworkStatusAction(value.opt(it), parameter) }
    is JSONObject -> {
        val subscription = value.optJSONObject("NetworkStatusSubscribe")
        if (subscription?.optString("parameter") == parameter) subscription
        else value.keys().asSequence()
            .firstNotNullOfOrNull { findNetworkStatusAction(value.opt(it), parameter) }
    }
    else -> null
}

internal fun NexaDevStateStore.mutateCollection(mutation: JSONObject, scope: String, locals: Map<String, Any>) {
    val name = mutation.optString("name")
    val rawArguments = mutation.optJSONArray("arguments") ?: JSONArray()
    val arguments = (0 until rawArguments.length()).map { evaluate(rawArguments.opt(it), locals, scope) }
    applyCollectionMutation(mutation, name, scope, arguments)
}

private fun NexaDevStateStore.applyCollectionMutation(
    mutation: JSONObject,
    name: String,
    scope: String,
    arguments: List<Any>,
) {
    when (mutation.optString("operation")) {
        "ArrayAppend" -> {
            val target = (state(name, scope) as? List<*>)?.toMutableList() ?: mutableListOf<Any?>()
            if (arguments.isNotEmpty()) target.add(arguments[0])
            setState(name, target, scope)
            moduleRevision++
        }
        "ArrayRemoveAt" -> {
            val target = (state(name, scope) as? List<*>)?.toMutableList() ?: mutableListOf<Any?>()
            val index = (arguments.firstOrNull() as? Number)?.toInt() ?: -1
            if (index in 0 until target.size) target.removeAt(index)
            setState(name, target, scope)
            moduleRevision++
        }
        "ArrayMove" -> {
            val target = (state(name, scope) as? List<*>)?.toMutableList() ?: mutableListOf<Any?>()
            val from = (arguments.getOrNull(0) as? Number)?.toInt() ?: -1
            val to = (arguments.getOrNull(1) as? Number)?.toInt() ?: -1
            if (from in target.indices && to in target.indices && from != to) {
                target.add(to, target.removeAt(from))
            }
            setState(name, target, scope)
            moduleRevision++
        }
        "ArrayMoveSubset" -> {
            val target = (state(name, scope) as? List<*>)?.toMutableList() ?: mutableListOf<Any?>()
            val subset = arguments.getOrNull(2) as? List<*> ?: return
            val from = (arguments.getOrNull(0) as? Number)?.toInt() ?: -1
            val to = (arguments.getOrNull(1) as? Number)?.toInt() ?: -1
            if (from in subset.indices && to in subset.indices && from != to) {
                val reordered = subset.toMutableList()
                reordered.add(to, reordered.removeAt(from))
                var cursor = 0
                for (index in target.indices) {
                    if (cursor < subset.size && devValueEquals(target[index], subset[cursor])) {
                        target[index] = reordered[cursor]
                        cursor++
                    }
                }
                if (cursor == subset.size) {
                    setState(name, target, scope)
                    moduleRevision++
                }
            }
        }
        "SetInsert", "SetRemove" -> {
            val target = (state(name, scope) as? Set<*>)?.filterNotNull()?.toMutableSet() ?: mutableSetOf()
            val item = arguments.firstOrNull()
            if (item != null) {
                if (mutation.optString("operation") == "SetInsert") target.add(item) else target.remove(item)
            }
            setState(name, target, scope)
            moduleRevision++
        }
        // A collection state keeps its identity: only the contents are
        // replaced, which is what the generated code does too.
        "Replace" -> {
            val replacement = arguments.firstOrNull()
            if (replacement != null) {
                setState(name, replacement, scope)
                moduleRevision++
            }
        }
        "MapSet", "MapRemove" -> {
            val target = (state(name, scope) as? Map<*, *>)?.mapKeys { stringify(it.key ?: JSONObject.NULL) }?.toMutableMap() ?: mutableMapOf()
            val key = arguments.firstOrNull()?.let { stringify(it) }
            if (key != null) {
                if (mutation.optString("operation") == "MapSet" && arguments.size > 1) {
                    target[key] = arguments[1]
                } else {
                    target.remove(key)
                }
            }
            setState(name, target, scope)
            moduleRevision++
        }
        "MapClear" -> {
            setState(name, emptyMap<String, Any?>(), scope)
            moduleRevision++
        }
    }
}

private fun devValueEquals(left: Any?, right: Any?): Boolean {
    if (left === right || left == right) return true
    if (left is JSONObject && right is JSONObject) {
        if (left.length() != right.length()) return false
        val keys = left.keys()
        while (keys.hasNext()) {
            val key = keys.next()
            if (!right.has(key) || !devValueEquals(left.opt(key), right.opt(key))) return false
        }
        return true
    }
    if (left is JSONArray && right is JSONArray) {
        if (left.length() != right.length()) return false
        return (0 until left.length()).all { devValueEquals(left.opt(it), right.opt(it)) }
    }
    return false
}

internal suspend fun NexaDevStateStore.performAsync(
    actions: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevActionFlow {
    for (index in 0 until actions.length()) {
        val raw = actions.opt(index)
        if (raw is String) {
            if (raw == "Break") return NexaDevActionFlow.Break
            if (raw == "Continue") return NexaDevActionFlow.Continue
            continue
        }
        val action = raw as? JSONObject ?: continue
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            try {
                val flow = performAsync(tryCatch.optJSONArray("body") ?: JSONArray(), scope, locals)
                if (flow != NexaDevActionFlow.Normal) return flow
            } catch (cancellation: CancellationException) {
                throw cancellation
            } catch (failure: NexaDevPluginFailure) {
                val caught = performPluginFailureCatchAsync(
                    failure,
                    tryCatch.optJSONArray("error_catches") ?: JSONArray(),
                    scope,
                    locals,
                )
                if (caught != null) {
                    if (caught != NexaDevActionFlow.Normal) return caught
                } else {
                    val catchBody = tryCatch.optJSONArray("catch_body")
                    if (catchBody != null) {
                        val flow = performAsync(catchBody, scope, locals)
                        if (flow != NexaDevActionFlow.Normal) return flow
                    } else {
                        android.util.Log.e("NexaDevRuntime", "Async dev action failed", failure)
                        throw failure
                    }
                }
            } catch (error: Throwable) {
                android.util.Log.e("NexaDevRuntime", "Async dev action failed", error)
                val catchBody = tryCatch.optJSONArray("catch_body")
                if (catchBody != null) {
                    val flow = performAsync(catchBody, scope, locals)
                    if (flow != NexaDevActionFlow.Normal) return flow
                } else {
                    throw error
                }
            }
            continue
        }
        val assign = action.optJSONObject("Assign")
        if (assign != null) {
            val name = assign.optString("name")
            val nextValue = evaluateAsync(assign.opt("value"), locals, scope)
            setState(name, nextValue, scope)
            moduleRevision++
            continue
        }
        val nativeAssignment = action.optJSONObject("NativePropertyAssign")
        if (nativeAssignment != null) {
            val receiver = evaluateAsync(nativeAssignment.opt("receiver"), locals, scope)
            val property = nativeAssignment.optString("property")
            val value = evaluateAsync(nativeAssignment.opt("value"), locals, scope)
            if (!NexaDevPluginBridge.writeInstanceProperty(receiver, property, value)) {
                android.util.Log.w("NexaDevRuntime", "Native property write is unsupported: $property")
            }
            continue
        }
        val subscription = action.optJSONObject("NativeEventSubscribe")
        if (subscription != null) {
            val receiver = evaluateAsync(subscription.opt("receiver"), locals, scope)
            val property = subscription.optString("property")
            if (!subscribeNativeEvent(
                receiver = receiver,
                property = property,
                actions = subscription.optJSONArray("actions") ?: JSONArray(),
                parameterNames = subscription.optJSONArray("parameters") ?: JSONArray(),
                scope = scope,
                locals = locals,
            )) {
                android.util.Log.w("NexaDevRuntime", "Native event subscription is unsupported: $property")
            }
            continue
        }
        val networkSubscription = action.optJSONObject("NetworkStatusSubscribe")
        if (networkSubscription != null) {
            val parameter = networkSubscription.optString("parameter")
            if (parameter.isNotEmpty()) {
                subscribeNetworkStatus(
                    parameter = parameter,
                    actions = networkSubscription.optJSONArray("actions") ?: JSONArray(),
                    scope = scope,
                    locals = locals,
                )
            }
            continue
        }
        val taskLaunch = action.optJSONObject("TaskLaunch")
        if (taskLaunch != null) {
            launchNativeTask(
                handle = taskLaunch.optString("handle").takeIf { taskLaunch.has("handle") && !taskLaunch.isNull("handle") },
                executor = taskLaunch.optString("executor"),
                actions = taskLaunch.optJSONArray("actions") ?: JSONArray(),
                scope = scope,
                locals = locals,
            )
            continue
        }
        val taskCancel = action.optJSONObject("TaskCancel")
        if (taskCancel != null) {
            cancelNativeTask(scope, taskCancel.optString("handle"))
            continue
        }
        val withAnimation = action.optJSONObject("WithAnimation")
        if (withAnimation != null) {
            val flow = performWithAnimation(withAnimation, scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            continue
        }
        val branch = action.optJSONObject("If")
        if (branch != null) {
            val conditionValue = evaluateAsync(branch.opt("condition"), locals, scope)
            val selected = if (truthy(conditionValue)) {
                branch.optJSONArray("then_branch")
            } else {
                branch.optJSONArray("else_branch")
            }
            val flow = performAsync(selected ?: JSONArray(), scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            continue
        }
        val expression = action.opt("Expression")
        if (expression != null) {
            evaluateAsync(expression, locals, scope)
            continue
        }
        val loop = action.optJSONObject("For")
        if (loop != null) {
            val name = loop.optString("name")
            val iterable = evaluateAsync(loop.opt("iterable"), locals, scope)
            val items = when (iterable) {
                is List<*> -> iterable
                is Set<*> -> iterable.toList()
                else -> emptyList<Any?>()
            }
            iteration@ for (item in items) {
                val nextLocals = locals.toMutableMap()
                nextLocals[name] = item.orNullValue()
                when (performAsync(loop.optJSONArray("body") ?: JSONArray(), scope, nextLocals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val mapLoop = action.optJSONObject("ForMap")
        if (mapLoop != null) {
            val keyName = mapLoop.optString("key_name")
            val valueName = mapLoop.optString("value_name")
            val iterable = evaluateAsync(mapLoop.opt("iterable"), locals, scope)
            val map = iterable as? Map<*, *> ?: emptyMap<Any?, Any?>()
            iteration@ for ((key, value) in map) {
                val nextLocals = locals.toMutableMap()
                nextLocals[keyName] = key.orNullValue()
                nextLocals[valueName] = value.orNullValue()
                when (performAsync(mapLoop.optJSONArray("body") ?: JSONArray(), scope, nextLocals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val whileLoop = action.optJSONObject("While")
        if (whileLoop != null) {
            var iterations = 0
            iteration@ while (
                iterations++ < 10_000 &&
                    truthy(evaluateAsync(whileLoop.opt("condition"), locals, scope))
            ) {
                when (performAsync(whileLoop.optJSONArray("body") ?: JSONArray(), scope, locals)) {
                    NexaDevActionFlow.Break -> break@iteration
                    NexaDevActionFlow.Continue, NexaDevActionFlow.Normal -> Unit
                }
            }
            continue
        }
        val mutation = action.optJSONObject("CollectionMutation")
        if (mutation != null) {
            val name = mutation.optString("name")
            val rawArguments = mutation.optJSONArray("arguments") ?: JSONArray()
            val arguments = ArrayList<Any>(rawArguments.length())
            for (argumentIndex in 0 until rawArguments.length()) {
                arguments += evaluateAsync(rawArguments.opt(argumentIndex), locals, scope)
            }
            applyCollectionMutation(mutation, name, scope, arguments)
            continue
        }
        if (action.has("Break")) return NexaDevActionFlow.Break
        if (action.has("Continue")) return NexaDevActionFlow.Continue
    }
    return NexaDevActionFlow.Normal
}

private fun NexaDevStateStore.performWithAnimation(
    block: JSONObject,
    scope: String,
    locals: Map<String, Any>,
): NexaDevActionFlow {
    val targets = block.optJSONArray("animated_states") ?: JSONArray()
    val names = (0 until targets.length()).mapNotNull { targets.optString(it).takeIf(String::isNotEmpty) }
    if (names.isEmpty()) {
        return performActions(block.optJSONArray("actions") ?: JSONArray(), scope, locals)
    }
    val previous = names.associateWith { state(it, scope) }
    for (name in names) setAnimationSpec(name, block.opt("animation"), scope)
    val flow = performActions(block.optJSONArray("actions") ?: JSONArray(), scope, locals)
    for (name in names) {
        val before = previous[name]
        val after = state(name, scope)
        val unchanged = if (before is Number && after is Number) {
            before.toDouble() == after.toDouble()
        } else {
            before == after
        }
        if (unchanged) clearAnimationSpec(name, scope)
    }
    return flow
}

internal fun NexaDevStateStore.performPluginFailureCatch(
    failure: NexaDevPluginFailure,
    arms: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevActionFlow? {
    for (index in 0 until arms.length()) {
        val arm = arms.optJSONObject(index) ?: continue
        if (arm.optString("namespace") != failure.namespace ||
            arm.optString("error_type") != failure.errorType ||
            arm.optString("variant") != failure.variant
        ) continue
        val catchLocals = locals.toMutableMap()
        val parameters = arm.optJSONArray("parameters") ?: JSONArray()
        for (parameterIndex in 0 until parameters.length()) {
            val tuple = parameters.optJSONArray(parameterIndex) ?: continue
            if (tuple.length() < 2) continue
            val binding = tuple.optString(0)
            val property = tuple.optString(1)
            catchLocals[binding] = failure.payload[property] ?: JSONObject.NULL
        }
        return performActions(arm.optJSONArray("body") ?: JSONArray(), scope, catchLocals)
    }
    return null
}

internal suspend fun NexaDevStateStore.performPluginFailureCatchAsync(
    failure: NexaDevPluginFailure,
    arms: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevActionFlow? {
    for (index in 0 until arms.length()) {
        val arm = arms.optJSONObject(index) ?: continue
        if (arm.optString("namespace") != failure.namespace ||
            arm.optString("error_type") != failure.errorType ||
            arm.optString("variant") != failure.variant
        ) continue
        val catchLocals = locals.toMutableMap()
        val parameters = arm.optJSONArray("parameters") ?: JSONArray()
        for (parameterIndex in 0 until parameters.length()) {
            val tuple = parameters.optJSONArray(parameterIndex) ?: continue
            if (tuple.length() < 2) continue
            val binding = tuple.optString(0)
            val property = tuple.optString(1)
            catchLocals[binding] = failure.payload[property] ?: JSONObject.NULL
        }
        return performAsync(arm.optJSONArray("body") ?: JSONArray(), scope, catchLocals)
    }
    return null
}

/** Interprets user class-function bodies, including returns nested in flow. */
internal suspend fun NexaDevStateStore.performFunctionAsync(
    actions: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevFunctionFlow {
    val functionLocals = locals.toMutableMap()
    for (index in 0 until actions.length()) {
        val action = actions.optJSONObject(index) ?: continue
        val binding = action.optJSONObject("Let")
        if (binding != null) {
            val name = binding.optString("name")
            functionLocals[name] = evaluateAsync(binding.opt("value"), functionLocals, scope)
            continue
        }
        val returned = action.optJSONObject("Return")
        if (returned != null) {
            return NexaDevFunctionFlow.Returned(evaluateAsync(returned.opt("value"), functionLocals, scope))
        }
        val branch = action.optJSONObject("If")
        if (branch != null) {
            val condition = truthy(evaluateAsync(branch.opt("condition"), functionLocals, scope))
            val selected = if (condition) branch.optJSONArray("then_branch") else branch.optJSONArray("else_branch")
            when (val flow = performFunctionAsync(selected ?: JSONArray(), scope, functionLocals)) {
                NexaDevFunctionFlow.Normal -> Unit
                else -> return flow
            }
            continue
        }
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            try {
                when (val flow = performFunctionAsync(tryCatch.optJSONArray("body") ?: JSONArray(), scope, functionLocals)) {
                    NexaDevFunctionFlow.Normal -> Unit
                    else -> return flow
                }
            } catch (cancellation: CancellationException) {
                throw cancellation
            } catch (failure: NexaDevPluginFailure) {
                val caught = performFunctionPluginFailureCatchAsync(
                    failure,
                    tryCatch.optJSONArray("error_catches") ?: JSONArray(),
                    scope,
                    functionLocals,
                )
                if (caught != null) {
                    when (caught) {
                        NexaDevFunctionFlow.Normal -> Unit
                        else -> return caught
                    }
                } else {
                    val catchBody = tryCatch.optJSONArray("catch_body")
                    if (catchBody != null) {
                        when (val flow = performFunctionAsync(catchBody, scope, functionLocals)) {
                            NexaDevFunctionFlow.Normal -> Unit
                            else -> return flow
                        }
                    } else {
                        throw failure
                    }
                }
            } catch (error: Throwable) {
                val catchBody = tryCatch.optJSONArray("catch_body")
                if (catchBody != null) {
                    when (val flow = performFunctionAsync(catchBody, scope, functionLocals)) {
                        NexaDevFunctionFlow.Normal -> Unit
                        else -> return flow
                    }
                } else {
                    throw error
                }
            }
            continue
        }
        val expression = action.opt("Expression")
        if (expression != null) {
            evaluateAsync(expression, functionLocals, scope)
            continue
        }
        val loop = action.optJSONObject("For")
        if (loop != null) {
            val name = loop.optString("name")
            val iterable = evaluateAsync(loop.opt("iterable"), functionLocals, scope)
            val values = when (iterable) {
                is List<*> -> iterable
                is Set<*> -> iterable.toList()
                else -> emptyList<Any?>()
            }
            for (value in values) {
                val iterationLocals = functionLocals.toMutableMap()
                iterationLocals[name] = value.orNullValue()
                when (val flow = performFunctionAsync(loop.optJSONArray("body") ?: JSONArray(), scope, iterationLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        val mapLoop = action.optJSONObject("ForMap")
        if (mapLoop != null) {
            val keyName = mapLoop.optString("key_name")
            val valueName = mapLoop.optString("value_name")
            val map = evaluateAsync(mapLoop.opt("iterable"), functionLocals, scope) as? Map<*, *> ?: emptyMap<Any?, Any?>()
            for ((key, value) in map) {
                val iterationLocals = functionLocals.toMutableMap()
                iterationLocals[keyName] = key.orNullValue()
                iterationLocals[valueName] = value.orNullValue()
                when (val flow = performFunctionAsync(mapLoop.optJSONArray("body") ?: JSONArray(), scope, iterationLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        val whileLoop = action.optJSONObject("While")
        if (whileLoop != null) {
            var iterations = 0
            while (iterations++ < 10_000 && truthy(evaluateAsync(whileLoop.opt("condition"), functionLocals, scope))) {
                when (val flow = performFunctionAsync(whileLoop.optJSONArray("body") ?: JSONArray(), scope, functionLocals)) {
                    NexaDevFunctionFlow.Normal, NexaDevFunctionFlow.Continue -> Unit
                    NexaDevFunctionFlow.Break -> break
                    is NexaDevFunctionFlow.Returned -> return flow
                }
            }
            continue
        }
        if (action.has("Break")) return NexaDevFunctionFlow.Break
        if (action.has("Continue")) return NexaDevFunctionFlow.Continue
    }
    return NexaDevFunctionFlow.Normal
}

private suspend fun NexaDevStateStore.performFunctionPluginFailureCatchAsync(
    failure: NexaDevPluginFailure,
    arms: JSONArray,
    scope: String,
    locals: Map<String, Any>,
): NexaDevFunctionFlow? {
    for (index in 0 until arms.length()) {
        val arm = arms.optJSONObject(index) ?: continue
        if (arm.optString("namespace") != failure.namespace ||
            arm.optString("error_type") != failure.errorType ||
            arm.optString("variant") != failure.variant
        ) continue
        val catchLocals = locals.toMutableMap()
        val parameters = arm.optJSONArray("parameters") ?: JSONArray()
        for (parameterIndex in 0 until parameters.length()) {
            val tuple = parameters.optJSONArray(parameterIndex) ?: continue
            if (tuple.length() < 2) continue
            val name = tuple.optString(0)
            val property = tuple.optString(1)
            catchLocals[name] = failure.payload[property] ?: JSONObject.NULL
        }
        return performFunctionAsync(arm.optJSONArray("body") ?: JSONArray(), scope, catchLocals)
    }
    return null
}
