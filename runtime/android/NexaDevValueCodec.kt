package __NEXA_PACKAGE__

import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject

/// Dynamic value codecs used only by the dev host's interpreter when it calls
/// a generic native plugin method. AOT calls still use generated typed codecs.
internal object NexaDevValueCodec {
    private class JsonFailure(val caseName: String) : RuntimeException(null, null, false, false)

    fun asSet(raw: Any?): Set<Any>? = when (raw) {
        is Set<*> -> raw.map { it ?: JSONObject.NULL }.toSet()
        is List<*> -> raw.map { it ?: JSONObject.NULL }.toSet()
        else -> null
    }

    fun asMap(raw: Any?): Map<Any, Any>? {
        val values = raw as? Map<*, *> ?: return null
        if (values.keys.any { it == null }) return null
        return values.entries.associate { it.key!! to (it.value ?: JSONObject.NULL) }
    }

    fun <Element> transformSet(raw: Any?, decode: (Any) -> Element?): Set<Element>? {
        val values: Collection<*> = when (raw) {
            is Set<*> -> raw
            is List<*> -> raw
            else -> return null
        }
        val decoded = LinkedHashSet<Element>()
        for (value in values) {
            val element = decode(value ?: JSONObject.NULL) ?: return null
            decoded.add(element)
        }
        return decoded
    }

    fun <Element> transformOptionalSet(raw: Any?, decode: (Any) -> Element?): Set<Element?>? {
        val values: Collection<*> = when (raw) {
            is Set<*> -> raw
            is List<*> -> raw
            else -> return null
        }
        val decoded = LinkedHashSet<Element?>(values.size)
        for (value in values) {
            if (value == null || value == JSONObject.NULL) {
                decoded.add(null)
            } else {
                decoded.add(decode(value) ?: return null)
            }
        }
        return decoded
    }

    fun <Element> transformArray(raw: Any?, decode: (Any) -> Element?): List<Element>? {
        val values = raw as? List<*> ?: return null
        val decoded = ArrayList<Element>(values.size)
        for (value in values) decoded += decode(value ?: JSONObject.NULL) ?: return null
        return decoded
    }

    fun <Element> transformOptionalArray(raw: Any?, decode: (Any) -> Element?): List<Element?>? {
        val values = raw as? List<*> ?: return null
        val decoded = ArrayList<Element?>(values.size)
        for (value in values) {
            if (value == null || value == JSONObject.NULL) {
                decoded += null
            } else {
                decoded += decode(value) ?: return null
            }
        }
        return decoded
    }

    fun <First, Second> transformPair(
        raw: Any?,
        decodeFirst: (Any) -> First?,
        decodeSecond: (Any) -> Second?,
    ): Pair<First, Second>? {
        if (raw is Pair<*, *>) {
            val first = decodeFirst(raw.first ?: JSONObject.NULL) ?: return null
            val second = decodeSecond(raw.second ?: JSONObject.NULL) ?: return null
            return Pair(first, second)
        }
        val elements = raw as? List<*> ?: return null
        if (elements.size != 2) return null
        val first = decodeFirst(elements[0] ?: JSONObject.NULL) ?: return null
        val second = decodeSecond(elements[1] ?: JSONObject.NULL) ?: return null
        return Pair(first, second)
    }

    fun <First, Second, Third> transformTriple(
        raw: Any?,
        decodeFirst: (Any) -> First?,
        decodeSecond: (Any) -> Second?,
        decodeThird: (Any) -> Third?,
    ): Triple<First, Second, Third>? {
        if (raw is Triple<*, *, *>) {
            val first = decodeFirst(raw.first ?: JSONObject.NULL) ?: return null
            val second = decodeSecond(raw.second ?: JSONObject.NULL) ?: return null
            val third = decodeThird(raw.third ?: JSONObject.NULL) ?: return null
            return Triple(first, second, third)
        }
        val elements = raw as? List<*> ?: return null
        if (elements.size != 3) return null
        val first = decodeFirst(elements[0] ?: JSONObject.NULL) ?: return null
        val second = decodeSecond(elements[1] ?: JSONObject.NULL) ?: return null
        val third = decodeThird(elements[2] ?: JSONObject.NULL) ?: return null
        return Triple(first, second, third)
    }

    fun <Key : Any, Value> transformMap(
        raw: Any?,
        decodeKey: (Any) -> Key?,
        decodeValue: (Any) -> Value?,
    ): Map<Key, Value>? {
        val values = raw as? Map<*, *> ?: return null
        if (values.keys.any { it == null }) return null
        val decoded = LinkedHashMap<Key, Value>(values.size)
        for ((rawKey, rawValue) in values) {
            val key = decodeKey(rawKey ?: return null) ?: return null
            val value = decodeValue(rawValue ?: JSONObject.NULL) ?: return null
            decoded[key] = value
        }
        return decoded
    }

    fun <Key : Any, Value> transformOptionalValueMap(
        raw: Any?,
        decodeKey: (Any) -> Key?,
        decodeValue: (Any) -> Value?,
    ): Map<Key, Value?>? {
        val values = raw as? Map<*, *> ?: return null
        if (values.keys.any { it == null }) return null
        val decoded = LinkedHashMap<Key, Value?>(values.size)
        for ((rawKey, rawValue) in values) {
            val key = decodeKey(rawKey ?: return null) ?: return null
            if (rawValue == null || rawValue == JSONObject.NULL) {
                decoded[key] = null
            } else {
                decoded[key] = decodeValue(rawValue) ?: return null
            }
        }
        return decoded
    }

    fun transformPairNullable(
        raw: Any?,
        optionalFirst: Boolean,
        decodeFirst: (Any) -> Any?,
        optionalSecond: Boolean,
        decodeSecond: (Any) -> Any?,
    ): Pair<Any?, Any?>? {
        val firstRaw: Any?
        val secondRaw: Any?
        when (raw) {
            is Pair<*, *> -> {
                firstRaw = raw.first
                secondRaw = raw.second
            }
            is List<*> -> {
                if (raw.size != 2) return null
                firstRaw = raw[0]
                secondRaw = raw[1]
            }
            else -> return null
        }
        val first = decodePart(firstRaw, optionalFirst, decodeFirst)
        if (first === InvalidDecode) return null
        val second = decodePart(secondRaw, optionalSecond, decodeSecond)
        if (second === InvalidDecode) return null
        return first to second
    }

    fun transformTripleNullable(
        raw: Any?,
        optionalFirst: Boolean,
        decodeFirst: (Any) -> Any?,
        optionalSecond: Boolean,
        decodeSecond: (Any) -> Any?,
        optionalThird: Boolean,
        decodeThird: (Any) -> Any?,
    ): Triple<Any?, Any?, Any?>? {
        val firstRaw: Any?
        val secondRaw: Any?
        val thirdRaw: Any?
        when (raw) {
            is Triple<*, *, *> -> {
                firstRaw = raw.first
                secondRaw = raw.second
                thirdRaw = raw.third
            }
            is List<*> -> {
                if (raw.size != 3) return null
                firstRaw = raw[0]
                secondRaw = raw[1]
                thirdRaw = raw[2]
            }
            else -> return null
        }
        val first = decodePart(firstRaw, optionalFirst, decodeFirst)
        if (first === InvalidDecode) return null
        val second = decodePart(secondRaw, optionalSecond, decodeSecond)
        if (second === InvalidDecode) return null
        val third = decodePart(thirdRaw, optionalThird, decodeThird)
        if (third === InvalidDecode) return null
        return Triple(first, second, third)
    }

    private fun decodePart(
        raw: Any?,
        optional: Boolean,
        decode: (Any) -> Any?,
    ): Any? {
        if (raw == null || raw == JSONObject.NULL) {
            return if (optional) null else InvalidDecode
        }
        return decode(raw) ?: InvalidDecode
    }

    private object InvalidDecode

    fun write(
        raw: Any?,
        type: Any?,
        writer: dev.nexa.core.NexaValueWriter,
        enumCases: Map<String, List<String>>,
    ): Boolean {
        val (kind, payload) = typeTag(type) ?: return false
        if (kind == "Optional") {
            val present = raw != null && raw != JSONObject.NULL
            writer.writeBool(present)
            if (!present) return true
            return write(raw, payload, writer, enumCases)
        }
        val value = raw ?: return false
        when (kind) {
            "Bool" -> writer.writeBool(value as? Boolean ?: return false)
            "String" -> writer.writeString(value as? String ?: return false)
            "Bytes" -> {
                val text = value as? String ?: return false
                val bytes = runCatching { Base64.decode(text, Base64.DEFAULT) }.getOrNull() ?: return false
                writer.writeBuffer(bytes)
            }
            "Numeric" -> {
                val name = payload as? String ?: return false
                val number = (value as? Number)?.toDouble() ?: value.toString().toDoubleOrNull() ?: return false
                when (name) {
                    "Int8" -> writer.writeInt8(number.toInt().toByte())
                    "Int16" -> writer.writeInt16(number.toInt().toShort())
                    "Int32" -> writer.writeInt32(number.toInt())
                    "Int64" -> writer.writeInt64(number.toLong())
                    "UInt8" -> writer.writeUInt8(number.toInt().toUByte())
                    "UInt16" -> writer.writeUInt16(number.toInt().toUShort())
                    "UInt32" -> writer.writeUInt32(number.toLong().toUInt())
                    "UInt64" -> writer.writeUInt64(number.toLong().toULong())
                    "Float32" -> writer.writeFloat32(number.toFloat())
                    "Float64" -> writer.writeFloat64(number)
                    else -> return false
                }
            }
            "Enum" -> {
                val name = payload as? String ?: return false
                val caseName = value as? String ?: return false
                val ordinal = enumCases[name]?.indexOf(caseName)?.takeIf { it >= 0 } ?: return false
                writer.writeInt32(ordinal)
            }
            "Result" -> {
                val types = payload as? JSONArray ?: return false
                if (types.length() != 2) return false
                val values = value as? Map<*, *> ?: return false
                val success = values["Ok"]
                if (success != null && success != JSONObject.NULL) {
                    writer.writeBool(true)
                    return write(success, types.opt(0), writer, enumCases)
                }
                val failure = values["Err"]
                if (failure != null && failure != JSONObject.NULL) {
                    writer.writeBool(false)
                    return write(failure, types.opt(1), writer, enumCases)
                }
                return false
            }
            "Struct" -> {
                val descriptor = payload as? JSONObject ?: return false
                val fields = descriptor.optJSONArray("fields") ?: return false
                val values = value as? Map<*, *> ?: return false
                for (index in 0 until fields.length()) {
                    val field = fields.optJSONArray(index) ?: return false
                    val name = field.optString(0)
                    if (!write(values[name], field.opt(1), writer, enumCases)) return false
                }
            }
            "Array" -> {
                val values = value as? List<*> ?: return false
                writer.writeCount(values.size)
                for (element in values) if (!write(element, payload, writer, enumCases)) return false
            }
            "Set" -> {
                val values = value as? Set<*> ?: return false
                val encoded = ArrayList<ByteArray>(values.size)
                for (element in values) {
                    val sink = dev.nexa.core.NexaValueWriter()
                    if (!write(element, payload, sink, enumCases)) return false
                    encoded += sink.toByteArray()
                }
                encoded.sortWith { left, right -> dev.nexa.core.nexaCompareBytes(left, right) }
                writer.writeCount(encoded.size)
                for (entry in encoded) writer.writeRaw(entry)
            }
            "Map" -> {
                val types = payload as? JSONArray ?: return false
                if (types.length() != 2) return false
                val values = value as? Map<*, *> ?: return false
                val encoded = ArrayList<Pair<ByteArray, ByteArray>>(values.size)
                for ((key, element) in values) {
                    val keySink = dev.nexa.core.NexaValueWriter()
                    val valueSink = dev.nexa.core.NexaValueWriter()
                    if (!write(key, types.opt(0), keySink, enumCases) ||
                        !write(element, types.opt(1), valueSink, enumCases)
                    ) return false
                    encoded += keySink.toByteArray() to valueSink.toByteArray()
                }
                encoded.sortWith { left, right -> dev.nexa.core.nexaCompareBytes(left.first, right.first) }
                writer.writeCount(encoded.size)
                for (entry in encoded) {
                    writer.writeRaw(entry.first)
                    writer.writeRaw(entry.second)
                }
            }
            "Pair" -> {
                val types = payload as? JSONArray ?: return false
                if (types.length() != 2) return false
                if (value is Pair<*, *>) {
                    if (!write(value.first, types.opt(0), writer, enumCases) ||
                        !write(value.second, types.opt(1), writer, enumCases)
                    ) return false
                } else {
                    val elements = value as? List<*> ?: return false
                    if (elements.size != 2) return false
                    if (!write(elements[0], types.opt(0), writer, enumCases) ||
                        !write(elements[1], types.opt(1), writer, enumCases)
                    ) return false
                }
            }
            "Triple" -> {
                val types = payload as? JSONArray ?: return false
                if (types.length() != 3) return false
                if (value is Triple<*, *, *>) {
                    if (!write(value.first, types.opt(0), writer, enumCases) ||
                        !write(value.second, types.opt(1), writer, enumCases) ||
                        !write(value.third, types.opt(2), writer, enumCases)
                    ) return false
                } else {
                    val elements = value as? List<*> ?: return false
                    if (elements.size != 3) return false
                    if (!write(elements[0], types.opt(0), writer, enumCases) ||
                        !write(elements[1], types.opt(1), writer, enumCases) ||
                        !write(elements[2], types.opt(2), writer, enumCases)
                    ) return false
                }
            }
            else -> return false
        }
        return true
    }

    fun read(
        type: Any?,
        reader: dev.nexa.core.NexaValueReader,
        enumCases: Map<String, List<String>>,
    ): Any? {
        val (kind, payload) = typeTag(type) ?: return null
        return when (kind) {
            "Optional" -> {
                val present = reader.readBool() ?: return null
                if (!present) JSONObject.NULL else read(payload, reader, enumCases) ?: return null
            }
            "Bool" -> reader.readBool()
            "String" -> reader.readString()
            "Bytes" -> reader.readBuffer()?.let { Base64.encodeToString(it, Base64.NO_WRAP) }
            "Numeric" -> when (payload as? String) {
                "Int8" -> reader.readInt8()
                "Int16" -> reader.readInt16()
                "Int32" -> reader.readInt32()
                "Int64" -> reader.readInt64()
                "UInt8" -> reader.readUInt8()
                "UInt16" -> reader.readUInt16()
                "UInt32" -> reader.readUInt32()
                "UInt64" -> reader.readUInt64()
                "Float32" -> reader.readFloat32()
                "Float64" -> reader.readFloat64()
                else -> null
            }
            "Enum" -> {
                val name = payload as? String ?: return null
                val ordinal = reader.readInt32() ?: return null
                enumCases[name]?.getOrNull(ordinal)
            }
            "Result" -> {
                val types = payload as? JSONArray ?: return null
                if (types.length() != 2) return null
                val isSuccess = reader.readBool() ?: return null
                if (isSuccess) {
                    val success = read(types.opt(0), reader, enumCases) ?: return null
                    mapOf("Ok" to success)
                } else {
                    val failure = read(types.opt(1), reader, enumCases) ?: return null
                    mapOf("Err" to failure)
                }
            }
            "Struct" -> {
                val descriptor = payload as? JSONObject ?: return null
                val fields = descriptor.optJSONArray("fields") ?: return null
                val values = linkedMapOf<String, Any>()
                for (index in 0 until fields.length()) {
                    val field = fields.optJSONArray(index) ?: return null
                    val value = read(field.opt(1), reader, enumCases) ?: return null
                    values[field.optString(0)] = value
                }
                values
            }
            "Array", "Set" -> {
                val count = reader.readCount()?.takeIf { it in 0..1_000_000 } ?: return null
                val values = ArrayList<Any>(count)
                repeat(count) { values += read(payload, reader, enumCases) ?: return null }
                if (kind == "Set") values.toSet() else values
            }
            "Map" -> {
                val types = payload as? JSONArray ?: return null
                if (types.length() != 2) return null
                val count = reader.readCount()?.takeIf { it in 0..1_000_000 } ?: return null
                val values = LinkedHashMap<Any, Any>(count)
                repeat(count) {
                    val key = read(types.opt(0), reader, enumCases) ?: return null
                    val value = read(types.opt(1), reader, enumCases) ?: return null
                    values[key] = value
                }
                values
            }
            "Pair" -> {
                val types = payload as? JSONArray ?: return null
                if (types.length() != 2) return null
                val first = read(types.opt(0), reader, enumCases) ?: return null
                val second = read(types.opt(1), reader, enumCases) ?: return null
                Pair(first, second)
            }
            "Triple" -> {
                val types = payload as? JSONArray ?: return null
                if (types.length() != 3) return null
                val first = read(types.opt(0), reader, enumCases) ?: return null
                val second = read(types.opt(1), reader, enumCases) ?: return null
                val third = read(types.opt(2), reader, enumCases) ?: return null
                Triple(first, second, third)
            }
            else -> null
        }
    }

    fun parseJson(raw: String, type: Any?, enumCases: Map<String, List<String>>): Map<String, Any> {
        return try {
            val tokener = org.json.JSONTokener(raw)
            val json = tokener.nextValue()
            if (tokener.nextClean() != '\u0000') jsonFailure("invalidJson")
            mapOf("Ok" to readJson(json, type, enumCases))
        } catch (failure: JsonFailure) {
            mapOf("Err" to failure.caseName)
        } catch (_: Exception) {
            mapOf("Err" to "invalidJson")
        }
    }

    fun stringifyJson(raw: Any?, type: Any?, enumCases: Map<String, List<String>>): String {
        return buildString {
            appendJson(raw, type, enumCases, this)
        }
    }

    private fun readJson(raw: Any?, type: Any?, enumCases: Map<String, List<String>>): Any {
        val (kind, payload) = typeTag(type) ?: jsonFailure("invalidValue")
        if (kind == "Optional") {
            if (raw == null || raw == JSONObject.NULL) return JSONObject.NULL
            return readJson(raw, payload, enumCases)
        }
        if (raw == null || raw == JSONObject.NULL) jsonFailure("typeMismatch")
        return when (kind) {
            "String" -> raw as? String ?: jsonFailure("typeMismatch")
            "Bool" -> raw as? Boolean ?: jsonFailure("typeMismatch")
            "Bytes" -> {
                val encoded = raw as? String ?: jsonFailure("typeMismatch")
                try {
                    Base64.encodeToString(Base64.decode(encoded, Base64.DEFAULT), Base64.NO_WRAP)
                } catch (_: IllegalArgumentException) {
                    jsonFailure("invalidValue")
                }
            }
            "Numeric" -> {
                val numeric = payload as? String ?: jsonFailure("invalidValue")
                val text = (raw as? Number)?.toString() ?: raw as? String ?: jsonFailure("typeMismatch")
                when (numeric) {
                    "Int8" -> text.toByteOrNull() ?: jsonFailure("invalidValue")
                    "Int16" -> text.toShortOrNull() ?: jsonFailure("invalidValue")
                    "Int32" -> text.toIntOrNull() ?: jsonFailure("invalidValue")
                    "Int64" -> text.toLongOrNull() ?: jsonFailure("invalidValue")
                    "UInt8" -> text.toUByteOrNull() ?: jsonFailure("invalidValue")
                    "UInt16" -> text.toUShortOrNull() ?: jsonFailure("invalidValue")
                    "UInt32" -> text.toUIntOrNull() ?: jsonFailure("invalidValue")
                    "UInt64" -> text.toULongOrNull() ?: jsonFailure("invalidValue")
                    "Float32" -> when (text) {
                        "Infinity" -> Float.POSITIVE_INFINITY
                        "-Infinity" -> Float.NEGATIVE_INFINITY
                        "NaN" -> Float.NaN
                        else -> text.toFloatOrNull() ?: jsonFailure("invalidValue")
                    }
                    "Float64" -> when (text) {
                        "Infinity" -> Double.POSITIVE_INFINITY
                        "-Infinity" -> Double.NEGATIVE_INFINITY
                        "NaN" -> Double.NaN
                        else -> text.toDoubleOrNull() ?: jsonFailure("invalidValue")
                    }
                    else -> jsonFailure("invalidValue")
                }
            }
            "Enum" -> {
                val enumName = payload as? String ?: jsonFailure("invalidValue")
                val caseName = raw as? String ?: jsonFailure("typeMismatch")
                if (caseName !in enumCases[enumName].orEmpty()) jsonFailure("invalidValue")
                caseName
            }
            "Array", "Set" -> {
                val values = raw as? JSONArray ?: jsonFailure("typeMismatch")
                val decoded = ArrayList<Any>(values.length())
                for (index in 0 until values.length()) {
                    decoded += readJson(values.opt(index), payload, enumCases)
                }
                if (kind == "Set") decoded.toCollection(LinkedHashSet()) else decoded
            }
            "Map" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                if (types.length() != 2 || typeTag(types.opt(0))?.first != "String") jsonFailure("invalidValue")
                val values = raw as? JSONObject ?: jsonFailure("typeMismatch")
                val decoded = LinkedHashMap<String, Any>(values.length())
                val keys = values.keys()
                while (keys.hasNext()) {
                    val key = keys.next()
                    decoded[key] = readJson(values.opt(key), types.opt(1), enumCases)
                }
                decoded
            }
            "Pair", "Triple" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                val values = raw as? JSONArray ?: jsonFailure("typeMismatch")
                val expectedCount = if (kind == "Pair") 2 else 3
                if (types.length() != expectedCount || values.length() != expectedCount) jsonFailure("invalidValue")
                (0 until expectedCount).map { index -> readJson(values.opt(index), types.opt(index), enumCases) }
            }
            "Result" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                val values = raw as? JSONObject ?: jsonFailure("typeMismatch")
                if (types.length() != 2 || values.length() != 1) jsonFailure("invalidValue")
                when {
                    values.has("success") -> mapOf("Ok" to readJson(values.opt("success"), types.opt(0), enumCases))
                    values.has("failure") -> mapOf("Err" to readJson(values.opt("failure"), types.opt(1), enumCases))
                    else -> jsonFailure("invalidValue")
                }
            }
            "Struct" -> {
                val descriptor = payload as? JSONObject ?: jsonFailure("invalidValue")
                val fields = descriptor.optJSONArray("fields") ?: jsonFailure("invalidValue")
                val values = raw as? JSONObject ?: jsonFailure("typeMismatch")
                val decoded = LinkedHashMap<String, Any>(fields.length())
                for (index in 0 until fields.length()) {
                    val field = fields.optJSONArray(index) ?: jsonFailure("invalidValue")
                    val name = field.optString(0)
                    val fieldType = field.opt(1)
                    if (!values.has(name) && typeTag(fieldType)?.first != "Optional") jsonFailure("missingField")
                    decoded[name] = if (values.has(name)) readJson(values.opt(name), fieldType, enumCases)
                    else JSONObject.NULL
                }
                decoded
            }
            else -> jsonFailure("invalidValue")
        }
    }

    private fun appendJson(raw: Any?, type: Any?, enumCases: Map<String, List<String>>, output: StringBuilder) {
        val (kind, payload) = typeTag(type) ?: jsonFailure("invalidValue")
        if (kind == "Optional") {
            if (raw == null || raw == JSONObject.NULL) output.append("null")
            else appendJson(raw, payload, enumCases, output)
            return
        }
        if (raw == null || raw == JSONObject.NULL) jsonFailure("invalidValue")
        when (kind) {
            "String" -> output.append(JSONObject.quote(raw as? String ?: jsonFailure("invalidValue")))
            "Bool" -> output.append((raw as? Boolean)?.toString() ?: jsonFailure("invalidValue"))
            "Bytes" -> {
                val encoded = raw as? String ?: jsonFailure("invalidValue")
                val normalized = try {
                    Base64.encodeToString(Base64.decode(encoded, Base64.DEFAULT), Base64.NO_WRAP)
                } catch (_: IllegalArgumentException) {
                    jsonFailure("invalidValue")
                }
                output.append(JSONObject.quote(normalized))
            }
            "Numeric" -> {
                val numeric = payload as? String ?: jsonFailure("invalidValue")
                val text = when (numeric) {
                    "Float32" -> (raw as? Number)?.toFloat()?.let { float ->
                        if (float.isFinite()) float.toString() else JSONObject.quote(if (float.isNaN()) "NaN" else if (float < 0) "-Infinity" else "Infinity")
                    }
                    "Float64" -> (raw as? Number)?.toDouble()?.let { number ->
                        if (number.isFinite()) number.toString() else JSONObject.quote(if (number.isNaN()) "NaN" else if (number < 0) "-Infinity" else "Infinity")
                    }
                    else -> (raw as? Number)?.toString()
                } ?: jsonFailure("invalidValue")
                output.append(text)
            }
            "Enum" -> {
                val enumName = payload as? String ?: jsonFailure("invalidValue")
                val caseName = raw as? String ?: jsonFailure("invalidValue")
                if (caseName !in enumCases[enumName].orEmpty()) jsonFailure("invalidValue")
                output.append(JSONObject.quote(caseName))
            }
            "Array", "Set" -> {
                val values: List<*> = when (raw) {
                    is List<*> -> raw
                    is Set<*> -> raw.toList()
                    else -> jsonFailure("invalidValue")
                }
                val entries = if (kind == "Set") values.map { value ->
                    val encoded = StringBuilder()
                    appendJson(value, payload, enumCases, encoded)
                    encoded.toString()
                }.sorted() else emptyList()
                output.append('[')
                if (kind == "Set") {
                    output.append(entries.joinToString(","))
                } else {
                    values.forEachIndexed { index, value ->
                        if (index > 0) output.append(',')
                        appendJson(value, payload, enumCases, output)
                    }
                }
                output.append(']')
            }
            "Map" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                if (types.length() != 2 || typeTag(types.opt(0))?.first != "String") jsonFailure("invalidValue")
                val values = raw as? Map<*, *> ?: jsonFailure("invalidValue")
                output.append('{')
                values.keys.map { it as? String ?: jsonFailure("invalidValue") }.sorted().forEachIndexed { index, key ->
                    if (index > 0) output.append(',')
                    output.append(JSONObject.quote(key)).append(':')
                    appendJson(values[key], types.opt(1), enumCases, output)
                }
                output.append('}')
            }
            "Pair", "Triple" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                val values = raw as? List<*> ?: jsonFailure("invalidValue")
                val expectedCount = if (kind == "Pair") 2 else 3
                if (types.length() != expectedCount || values.size != expectedCount) jsonFailure("invalidValue")
                output.append('[')
                values.forEachIndexed { index, value ->
                    if (index > 0) output.append(',')
                    appendJson(value, types.opt(index), enumCases, output)
                }
                output.append(']')
            }
            "Result" -> {
                val types = payload as? JSONArray ?: jsonFailure("invalidValue")
                val values = raw as? Map<*, *> ?: jsonFailure("invalidValue")
                if (types.length() != 2) jsonFailure("invalidValue")
                val (caseName, child, childType) = when {
                    values.containsKey("Ok") -> Triple("success", values["Ok"], types.opt(0))
                    values.containsKey("Err") -> Triple("failure", values["Err"], types.opt(1))
                    else -> jsonFailure("invalidValue")
                }
                output.append('{').append(JSONObject.quote(caseName)).append(':')
                appendJson(child, childType, enumCases, output)
                output.append('}')
            }
            "Struct" -> {
                val descriptor = payload as? JSONObject ?: jsonFailure("invalidValue")
                val fields = descriptor.optJSONArray("fields") ?: jsonFailure("invalidValue")
                val values = raw as? Map<*, *> ?: jsonFailure("invalidValue")
                output.append('{')
                for (index in 0 until fields.length()) {
                    val field = fields.optJSONArray(index) ?: jsonFailure("invalidValue")
                    if (index > 0) output.append(',')
                    val name = field.optString(0)
                    output.append(JSONObject.quote(name)).append(':')
                    appendJson(values[name] ?: if (typeTag(field.opt(1))?.first == "Optional") JSONObject.NULL else jsonFailure("invalidValue"), field.opt(1), enumCases, output)
                }
                output.append('}')
            }
            else -> jsonFailure("invalidValue")
        }
    }

    private fun jsonFailure(caseName: String): Nothing = throw JsonFailure(caseName)

    private fun typeTag(raw: Any?): Pair<String, Any?>? {
        return when (raw) {
            is String -> raw to null
            is JSONObject -> {
                if (raw.has("ty")) return typeTag(raw.opt("ty"))
                val keys = raw.keys()
                if (!keys.hasNext()) null else keys.next().let { it to raw.opt(it) }
            }
            is Map<*, *> -> {
                if (raw.containsKey("ty")) return typeTag(raw["ty"])
                val entry = raw.entries.firstOrNull() ?: return null
                (entry.key as? String)?.let { it to entry.value }
            }
            else -> null
        }
    }
}
