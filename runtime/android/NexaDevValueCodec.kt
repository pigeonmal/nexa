package __NEXA_PACKAGE__

import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject

/// Dynamic value codecs used only by the dev host's interpreter when it calls
/// a generic native plugin method. AOT calls still use generated typed codecs.
internal object NexaDevValueCodec {
    fun asSet(raw: Any?): Set<Any>? = when (raw) {
        is Set<*> -> raw.filterNotNull().toSet()
        is List<*> -> raw.filterNotNull().toSet()
        else -> null
    }

    fun asMap(raw: Any?): Map<Any, Any>? {
        val values = raw as? Map<*, *> ?: return null
        if (values.keys.any { it == null } || values.values.any { it == null }) return null
        return values.entries.associate { it.key!! to it.value!! }
    }

    fun <Element> transformSet(raw: Any?, decode: (Any) -> Element?): Set<Element>? {
        val values: Iterable<*> = when (raw) {
            is Set<*> -> raw
            is List<*> -> raw
            else -> return null
        }
        val decoded = LinkedHashSet<Element>()
        for (value in values) {
            val element = decode(value ?: return null) ?: return null
            decoded.add(element)
        }
        return decoded
    }

    fun <Key : Any, Value> transformMap(
        raw: Any?,
        decodeKey: (Any) -> Key?,
        decodeValue: (Any) -> Value?,
    ): Map<Key, Value>? {
        val values = asMap(raw) ?: return null
        val decoded = LinkedHashMap<Key, Value>(values.size)
        for ((rawKey, rawValue) in values) {
            val key = decodeKey(rawKey) ?: return null
            val value = decodeValue(rawValue) ?: return null
            decoded[key] = value
        }
        return decoded
    }

    fun write(
        raw: Any?,
        type: Any?,
        writer: dev.nexa.core.NexaValueWriter,
        enumCases: Map<String, List<String>>,
    ): Boolean {
        val value = raw ?: return false
        val (kind, payload) = typeTag(type) ?: return false
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
            else -> null
        }
    }

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
