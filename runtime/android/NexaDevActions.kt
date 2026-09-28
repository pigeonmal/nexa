package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import java.lang.ref.WeakReference

internal enum class NexaDevActionFlow { Normal, Break, Continue }

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

internal fun NexaDevStateStore.refreshNativeEventSubscriptions(module: JSONObject) {
    if (nativeEventSubscriptions.isEmpty()) return
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
}

internal fun NexaDevStateStore.clearNativeEventSubscriptions(scope: String? = null) {
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
        "SetInsert", "SetRemove" -> {
            val target = (state(name, scope) as? Set<*>)?.map { stringify(it ?: JSONObject.NULL) }?.toMutableSet() ?: mutableSetOf()
            val item = arguments.firstOrNull()?.let { stringify(it) }
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
    }
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
