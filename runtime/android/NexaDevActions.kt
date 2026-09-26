package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject
import kotlinx.coroutines.CancellationException

internal enum class NexaDevActionFlow { Normal, Break, Continue }

internal fun Any?.orNullValue(): Any = this ?: JSONObject.NULL

internal fun NexaDevStateStore.perform(actions: JSONArray, scope: String, locals: Map<String, Any>) {
    performActions(actions, scope, locals)
}

internal fun NexaDevStateStore.performActions(actions: JSONArray, scope: String, locals: Map<String, Any>): NexaDevActionFlow {
    for (index in 0 until actions.length()) {
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
        val mutation = action.optJSONObject("CollectionMutation")
        if (mutation != null) {
            mutateCollection(mutation, scope, locals)
            continue
        }
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            val flow = performActions(tryCatch.optJSONArray("body") ?: JSONArray(), scope, locals)
            if (flow != NexaDevActionFlow.Normal) return flow
            continue
        }
        if (action.has("Break")) return NexaDevActionFlow.Break
        if (action.has("Continue")) return NexaDevActionFlow.Continue
    }
    return NexaDevActionFlow.Normal
}

internal fun NexaDevStateStore.mutateCollection(mutation: JSONObject, scope: String, locals: Map<String, Any>) {
    val name = mutation.optString("name")
    val rawArguments = mutation.optJSONArray("arguments") ?: JSONArray()
    val arguments = (0 until rawArguments.length()).map { evaluate(rawArguments.opt(it), locals, scope) }
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

internal suspend fun NexaDevStateStore.performAsync(actions: JSONArray, scope: String, locals: Map<String, Any>) {
    for (index in 0 until actions.length()) {
        val action = actions.optJSONObject(index) ?: continue
        val tryCatch = action.optJSONObject("TryCatch")
        if (tryCatch != null) {
            try {
                performAsync(tryCatch.optJSONArray("body") ?: JSONArray(), scope, locals)
            } catch (cancellation: CancellationException) {
                throw cancellation
            } catch (error: Throwable) {
                android.util.Log.e("NexaDevRuntime", "Async dev action failed", error)
                val catchBody = tryCatch.optJSONArray("catch_body")
                if (catchBody != null) {
                    performAsync(catchBody, scope, locals)
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
        val branch = action.optJSONObject("If")
        if (branch != null) {
            val conditionValue = evaluateAsync(branch.opt("condition"), locals, scope)
            val selected = if (truthy(conditionValue)) {
                branch.optJSONArray("then_branch")
            } else {
                branch.optJSONArray("else_branch")
            }
            performAsync(selected ?: JSONArray(), scope, locals)
            continue
        }
        val expression = action.opt("Expression")
        if (expression != null) {
            evaluateAsync(expression, locals, scope)
        }
    }
}
