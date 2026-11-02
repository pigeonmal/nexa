package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject

/**
 * A validated, side-effect-free candidate for one Dev IR patch.
 *
 * Preparation runs on the WebSocket worker against a frozen module snapshot.
 * It never mutates the installed module: changed branches are copied, so a
 * styling patch costs work proportional to the patched path rather than the
 * whole module. The candidate is committed on the main thread and rejected as
 * a whole if any operation is invalid or conflicting.
 */
internal class NexaDevPreparedPatch(
    val baseModule: JSONObject,
    val candidate: JSONObject,
    val presentationOnly: Boolean,
    val functionBodiesOnly: Boolean,
    val screenIndices: Set<Int>?,
    val operationCount: Int,
)

internal object NexaDevJsonPatch {
    private enum class Kind { Set, Remove }

    private class Operation(val kind: Kind, val segments: List<String>, val value: Any?)

    private sealed class Step {
        class Ok(val value: Any) : Step()
        object Invalid : Step()
    }

    /** Returns a prepared patch, or `null` when the patch must be rejected. */
    fun prepare(base: JSONObject, operations: JSONArray): NexaDevPreparedPatch? {
        val parsed = ArrayList<Operation>(operations.length())
        val occupied = ArrayList<List<String>>(operations.length())
        for (index in 0 until operations.length()) {
            val operation = operations.optJSONObject(index) ?: return reject("operation $index is not an object")
            val kind = when (operation.optString("op")) {
                "set" -> Kind.Set
                "remove" -> Kind.Remove
                else -> return reject("operation $index has unsupported op ${operation.optString("op")}")
            }
            val segments = parsePointer(operation.optString("path"))
                ?: return reject("operation $index has an invalid JSON pointer")
            if (segments.isEmpty()) {
                if (kind != Kind.Set) return reject("operation $index cannot remove root module")
                if (operations.length() > 1) return reject("operation $index cannot batch root replacement with other operations")
                val rootVal = operation.opt("value") as? JSONObject
                    ?: return reject("operation $index root replacement value is not a JSON object")
                if (!validModuleShape(rootVal)) {
                    return reject("operation $index root replacement value has invalid module shape")
                }
            }
            if (occupied.any { conflicts(it, segments) }) {
                return reject("operation $index conflicts with an earlier path")
            }
            occupied.add(segments)
            parsed.add(Operation(kind, segments, operation.opt("value")))
        }
        if (parsed.isEmpty()) return reject("patch has no operations")

        var candidate: Any = base
        for (operation in parsed) {
            candidate = if (operation.segments.isEmpty()) {
                operation.value as JSONObject
            } else {
                when (val step = update(candidate, operation, 0)) {
                    is Step.Ok -> step.value
                    Step.Invalid -> return reject("operation on ${operation.segments.joinToString("/")} cannot be applied")
                }
            }
        }
        val candidateObject = candidate as? JSONObject ?: return reject("candidate root is not an object")
        if (!validModuleShape(candidateObject)) return reject("candidate module shape is invalid")

        val presentationOnly = parsed.isNotEmpty() && parsed.all { operation ->
            val bodyIndex = when {
                operation.segments.firstOrNull() == "body" -> 0
                operation.segments.firstOrNull() == "screens" && operation.segments.getOrNull(2) == "body" -> 2
                operation.segments.firstOrNull() == "components" && operation.segments.getOrNull(2) == "body" -> 2
                else -> -1
            }
            val nodePropertiesStart = bodyIndex + 3
            bodyIndex >= 0 &&
                operation.segments.size >= bodyIndex + 4 &&
                operation.segments.drop(nodePropertiesStart)
                    .any { it in NEXA_DEV_PRESENTATION_PATCH_FIELDS }
        }
        val functionBodiesOnly = !presentationOnly && parsed.isNotEmpty() && parsed.all { operation ->
            operation.segments.size >= 3 &&
                operation.segments[0] == "functions" &&
                (operation.segments[2] == "body" || operation.segments[2] == "body_actions")
        }
        val screenIndices: Set<Int>? = if (!presentationOnly && !functionBodiesOnly) {
            val indices = mutableSetOf<Int>()
            var screenScoped = true
            for (operation in parsed) {
                if (operation.segments.size < 2 || operation.segments[0] != "screens") {
                    screenScoped = false
                    break
                }
                val value = operation.segments[1].toIntOrNull()
                if (value == null) {
                    screenScoped = false
                    break
                }
                indices.add(value)
            }
            indices.takeIf { screenScoped }
        } else {
            null
        }

        return NexaDevPreparedPatch(
            baseModule = base,
            candidate = candidateObject,
            presentationOnly = presentationOnly,
            functionBodiesOnly = functionBodiesOnly,
            screenIndices = screenIndices,
            operationCount = parsed.size,
        )
    }

    private fun reject(reason: String): NexaDevPreparedPatch? {
        android.util.Log.w("NexaDevRuntime", "Rejected Dev IR patch: $reason")
        return null
    }

    private fun conflicts(existing: List<String>, incoming: List<String>): Boolean {
        if (existing.isEmpty() || incoming.isEmpty()) return true
        val shared = minOf(existing.size, incoming.size)
        for (index in 0 until shared) {
            if (existing[index] != incoming[index]) return false
        }
        // Equal paths or a parent/child pair would make the result depend on
        // operation order and could resurrect a removed subtree.
        return true
    }

    private fun parsePointer(raw: String): List<String>? {
        if (raw.isEmpty()) return emptyList()
        if (raw[0] != '/') return null
        val segments = raw.substring(1).split('/')
        val decoded = ArrayList<String>(segments.size)
        for (segment in segments) {
            if (segment.isEmpty()) return null
            val value = StringBuilder(segment.length)
            var index = 0
            while (index < segment.length) {
                val character = segment[index]
                if (character == '~') {
                    if (index + 1 >= segment.length) return null
                    when (segment[index + 1]) {
                        '0' -> value.append('~')
                        '1' -> value.append('/')
                        else -> return null
                    }
                    index += 2
                } else {
                    value.append(character)
                    index += 1
                }
            }
            decoded.add(value.toString())
        }
        return decoded
    }

    private fun update(node: Any, operation: Operation, depth: Int): Step {
        val key = operation.segments[depth]
        val leaf = depth == operation.segments.size - 1
        return when (node) {
            is JSONObject -> {
                val copy = shallowCopyObject(node)
                if (leaf) {
                    when (operation.kind) {
                        Kind.Set -> {
                            copy.put(key, operation.value ?: JSONObject.NULL)
                            Step.Ok(copy)
                        }
                        Kind.Remove -> {
                            if (!node.has(key)) return Step.Invalid
                            copy.remove(key)
                            Step.Ok(copy)
                        }
                    }
                } else {
                    val child = node.opt(key) ?: return Step.Invalid
                    when (val step = update(child, operation, depth + 1)) {
                        is Step.Ok -> {
                            copy.put(key, step.value)
                            Step.Ok(copy)
                        }
                        Step.Invalid -> Step.Invalid
                    }
                }
            }
            is JSONArray -> {
                if (key == "-") {
                    if (leaf && operation.kind == Kind.Set) {
                        val copy = shallowCopyArray(node)
                        copy.put(operation.value ?: JSONObject.NULL)
                        return Step.Ok(copy)
                    }
                    return Step.Invalid
                }
                val arrayIndex = key.toIntOrNull() ?: return Step.Invalid
                if (leaf) {
                    when (operation.kind) {
                        Kind.Set -> {
                            if (arrayIndex in 0 until node.length()) {
                                val copy = shallowCopyArray(node)
                                copy.put(arrayIndex, operation.value ?: JSONObject.NULL)
                                Step.Ok(copy)
                            } else if (arrayIndex == node.length()) {
                                val copy = shallowCopyArray(node)
                                copy.put(operation.value ?: JSONObject.NULL)
                                Step.Ok(copy)
                            } else {
                                Step.Invalid
                            }
                        }
                        Kind.Remove -> {
                            if (arrayIndex in 0 until node.length()) {
                                val copy = JSONArray()
                                for (i in 0 until node.length()) {
                                    if (i != arrayIndex) {
                                        copy.put(node.get(i))
                                    }
                                }
                                Step.Ok(copy)
                            } else {
                                Step.Invalid
                            }
                        }
                    }
                } else {
                    if (arrayIndex !in 0 until node.length()) return Step.Invalid
                    val child = node.opt(arrayIndex) ?: return Step.Invalid
                    val copy = shallowCopyArray(node)
                    when (val step = update(child, operation, depth + 1)) {
                        is Step.Ok -> {
                            copy.put(arrayIndex, step.value)
                            Step.Ok(copy)
                        }
                        Step.Invalid -> Step.Invalid
                    }
                }
            }
            else -> Step.Invalid
        }
    }

    private fun shallowCopyObject(value: JSONObject): JSONObject {
        val copy = JSONObject()
        val keys = value.keys()
        while (keys.hasNext()) {
            val key = keys.next()
            copy.put(key, value.opt(key))
        }
        return copy
    }

    private fun shallowCopyArray(value: JSONArray): JSONArray {
        val copy = JSONArray()
        for (index in 0 until value.length()) copy.put(value.opt(index))
        return copy
    }

    private fun validModuleShape(module: JSONObject): Boolean {
        if (module.optString("app_name").isEmpty()) return false
        if (module.optJSONArray("body") == null) return false
        if (module.optJSONArray("screens") == null) return false
        if (module.optJSONArray("states") == null) return false
        if (module.optJSONArray("functions") == null) return false
        return true
    }
}
