import Foundation

/// Hashable wrapper for values stored in DevRuntime sets and generic maps.
/// AOT values remain their generated concrete types; this only bridges the
/// JSON-shaped interpreter values to generic plugin methods in a dev host.
struct NexaDevHashableValue: Hashable {
    let value: Any
    private let key: String
    var stableOrderKey: String { key }

    init(_ value: Any) {
        if let wrapped = value as? NexaDevHashableValue {
            self = wrapped
            return
        }
        self.value = value
        key = Self.stableKey(value)
    }

    static func == (lhs: Self, rhs: Self) -> Bool { lhs.key == rhs.key }

    func hash(into hasher: inout Hasher) { hasher.combine(key) }

    private static func stableKey(_ value: Any) -> String {
        if value is NSNull { return "null" }
        if let value = value as? String { return "s:\(value.utf8.count):\(value)" }
        if let value = value as? NSNumber { return "n:\(String(cString: value.objCType)):\(value.stringValue)" }
        if let values = value as? [Any] {
            return "a:[\(values.map(stableKey).joined(separator: ","))]"
        }
        if let values = value as? [String: Any] {
            return "o:{\(values.keys.sorted().map { key in "\(stableKey(key)):\(stableKey(values[key] ?? NSNull()))" }.joined(separator: ","))}"
        }
        if let values = value as? [AnyHashable: Any] {
            return "m:{\(values.map { (stableKey($0.key.base), stableKey($0.value)) }.sorted { $0.0 < $1.0 }.map { "\($0.0):\($0.1)" }.joined(separator: ","))}"
        }
        return "v:\(String(reflecting: value))"
    }
}

/// Dynamic value codecs for generic plugin calls in the debug host. The type
/// descriptor is serialized from the already type-checked IR; dispatch stays
/// explicit and does not use runtime reflection.
enum NexaDevValueCodec {
    /// Wraps interpreter values in a concrete, copyable type so Swift does not
    /// have to reabstract an `(Any, Writer)` closure to a plugin's generic
    /// codec closure. That reabstraction currently crashes Swift 6.4 SILGen.
    static func box(_ raw: Any?) -> NexaDevHashableValue? {
        guard let raw, !(raw is NSNull) else { return nil }
        return NexaDevHashableValue(raw)
    }

    static func transformArray<Element>(
        _ raw: Any?,
        decode: (Any) -> Element?
    ) -> [Element]? {
        guard let values = raw as? [Any] else { return nil }
        var decoded: [Element] = []
        decoded.reserveCapacity(values.count)
        for value in values {
            guard let value = decode(value) else { return nil }
            decoded.append(value)
        }
        return decoded
    }

    static func transformOptionalArray<Element>(
        _ raw: Any?,
        decode: (Any) -> Element?
    ) -> [Element?]? {
        guard let values = raw as? [Any] else { return nil }
        var decoded: [Element?] = []
        decoded.reserveCapacity(values.count)
        for value in values {
            if value is NSNull {
                decoded.append(nil)
            } else {
                guard let element = decode(value) else { return nil }
                decoded.append(element)
            }
        }
        return decoded
    }

    static func transformPair<First, Second>(
        _ raw: Any?,
        decodeFirst: (Any) -> First?,
        decodeSecond: (Any) -> Second?
    ) -> (First, Second)? {
        if let values = raw as? [Any], values.count == 2 {
            guard let first = decodeFirst(values[0]), let second = decodeSecond(values[1]) else {
                return nil
            }
            return (first, second)
        }
        if let pair = raw as? (Any, Any),
           let first = decodeFirst(pair.0),
           let second = decodeSecond(pair.1) {
            return (first, second)
        }
        return nil
    }

    static func transformTriple<First, Second, Third>(
        _ raw: Any?,
        decodeFirst: (Any) -> First?,
        decodeSecond: (Any) -> Second?,
        decodeThird: (Any) -> Third?
    ) -> (First, Second, Third)? {
        if let values = raw as? [Any], values.count == 3 {
            guard let first = decodeFirst(values[0]),
                  let second = decodeSecond(values[1]),
                  let third = decodeThird(values[2])
            else { return nil }
            return (first, second, third)
        }
        if let triple = raw as? (Any, Any, Any),
           let first = decodeFirst(triple.0),
           let second = decodeSecond(triple.1),
           let third = decodeThird(triple.2) {
            return (first, second, third)
        }
        return nil
    }

    static func transformSet<Element: Hashable>(
        _ raw: Any?,
        decode: (Any) -> Element?
    ) -> Set<Element>? {
        guard let values = asSet(raw) else { return nil }
        var decoded = Set<Element>()
        decoded.reserveCapacity(values.count)
        for value in values {
            guard let element = decode(value.value) else { return nil }
            decoded.insert(element)
        }
        return decoded
    }

    static func transformMap<Key: Hashable, Value>(
        _ raw: Any?,
        decodeKey: (Any) -> Key?,
        decodeValue: (Any) -> Value?
    ) -> [Key: Value]? {
        guard let values = asMap(raw) else { return nil }
        var decoded: [Key: Value] = [:]
        decoded.reserveCapacity(values.count)
        for (key, value) in values {
            guard let key = decodeKey(key.base), let value = decodeValue(value) else {
                return nil
            }
            decoded[key] = value
        }
        return decoded
    }

    static func asSet(_ raw: Any?) -> Set<NexaDevHashableValue>? {
        if let values = raw as? Set<NexaDevHashableValue> { return values }
        if let values = raw as? Set<AnyHashable> { return Set(values.map { NexaDevHashableValue($0.base) }) }
        if let values = raw as? Set<String> { return Set(values.map { NexaDevHashableValue($0) }) }
        if let values = raw as? [Any] { return Set(values.map(NexaDevHashableValue.init)) }
        return nil
    }

    static func asMap(_ raw: Any?) -> [AnyHashable: Any]? {
        if let values = raw as? [AnyHashable: Any] { return values }
        if let values = raw as? [String: Any] {
            return Dictionary(uniqueKeysWithValues: values.map { (AnyHashable($0.key), $0.value) })
        }
        return nil
    }

    static func write(
        _ raw: Any,
        type: Any,
        into writer: NexaValueWriter,
        enumCases: [String: [String]]
    ) -> Bool {
        let value = (raw as? NexaDevHashableValue)?.value ?? raw
        guard let (kind, payload) = typeTag(type) else { return false }
        switch kind {
        case "Bool":
            guard let number = value as? NSNumber else { return false }
            writer.writeBool(number.boolValue)
        case "String":
            guard let value = value as? String else { return false }
            writer.writeString(value)
        case "Bytes":
            guard let text = value as? String, let bytes = Data(base64Encoded: text) else { return false }
            writer.writeBuffer(bytes)
        case "Numeric":
            guard let name = payload as? String, let number = numberValue(value) else { return false }
            switch name {
            case "Int8": writer.writeInt8(Int8(truncatingIfNeeded: number.int64Value))
            case "Int16": writer.writeInt16(Int16(truncatingIfNeeded: number.int64Value))
            case "Int32": writer.writeInt32(Int32(truncatingIfNeeded: number.int64Value))
            case "Int64": writer.writeInt64(number.int64Value)
            case "UInt8": writer.writeUint8(UInt8(truncatingIfNeeded: number.uint64Value))
            case "UInt16": writer.writeUint16(UInt16(truncatingIfNeeded: number.uint64Value))
            case "UInt32": writer.writeUint32(UInt32(truncatingIfNeeded: number.uint64Value))
            case "UInt64": writer.writeUint64(number.uint64Value)
            case "Float32": writer.writeFloat32(number.floatValue)
            case "Float64": writer.writeFloat64(number.doubleValue)
            default: return false
            }
        case "Enum":
            guard let name = payload as? String,
                  let caseName = value as? String,
                  let ordinal = enumCases[name]?.firstIndex(of: caseName)
            else { return false }
            writer.writeInt32(Int32(ordinal))
        case "Struct":
            guard let fields = (payload as? [String: Any])?["fields"] as? [[Any]],
                  let values = value as? [String: Any]
            else { return false }
            for field in fields where field.count == 2 {
                guard let name = field[0] as? String,
                      let fieldValue = values[name],
                      write(fieldValue, type: field[1], into: writer, enumCases: enumCases)
                else { return false }
            }
        case "Array":
            guard let values = value as? [Any] else { return false }
            writer.writeCount(values.count)
            for element in values where !write(element, type: payload as Any, into: writer, enumCases: enumCases) {
                return false
            }
        case "Set":
            guard let values = asSet(value) else { return false }
            var encoded: [Data] = []
            encoded.reserveCapacity(values.count)
            for element in values {
                let sink = NexaValueWriter()
                guard write(element.value, type: payload as Any, into: sink, enumCases: enumCases) else { return false }
                encoded.append(sink.data)
            }
            encoded.sort { $0.lexicographicallyPrecedes($1) }
            writer.writeCount(encoded.count)
            for entry in encoded { writer.writeRaw(entry) }
        case "Map":
            guard let types = payload as? [Any], types.count == 2,
                  let values = asMap(value)
            else { return false }
            var encoded: [(Data, Data)] = []
            encoded.reserveCapacity(values.count)
            for (key, element) in values {
                let keyWriter = NexaValueWriter()
                let valueWriter = NexaValueWriter()
                guard write(key.base, type: types[0], into: keyWriter, enumCases: enumCases),
                      write(element, type: types[1], into: valueWriter, enumCases: enumCases)
                else { return false }
                encoded.append((keyWriter.data, valueWriter.data))
            }
            encoded.sort { $0.0.lexicographicallyPrecedes($1.0) }
            writer.writeCount(encoded.count)
            for entry in encoded {
                writer.writeRaw(entry.0)
                writer.writeRaw(entry.1)
            }
        case "Pair":
            guard let types = payload as? [Any], types.count == 2 else { return false }
            if let pair = value as? (Any, Any) {
                return write(pair.0, type: types[0], into: writer, enumCases: enumCases)
                    && write(pair.1, type: types[1], into: writer, enumCases: enumCases)
            }
            guard let values = value as? [Any], values.count == 2 else { return false }
            return write(values[0], type: types[0], into: writer, enumCases: enumCases)
                && write(values[1], type: types[1], into: writer, enumCases: enumCases)
        case "Triple":
            guard let types = payload as? [Any], types.count == 3 else { return false }
            if let triple = value as? (Any, Any, Any) {
                return write(triple.0, type: types[0], into: writer, enumCases: enumCases)
                    && write(triple.1, type: types[1], into: writer, enumCases: enumCases)
                    && write(triple.2, type: types[2], into: writer, enumCases: enumCases)
            }
            guard let values = value as? [Any], values.count == 3 else { return false }
            return write(values[0], type: types[0], into: writer, enumCases: enumCases)
                && write(values[1], type: types[1], into: writer, enumCases: enumCases)
                && write(values[2], type: types[2], into: writer, enumCases: enumCases)
        default:
            return false
        }
        return true
    }

    static func read(
        type: Any,
        from reader: NexaValueReader,
        enumCases: [String: [String]]
    ) -> Any? {
        guard let (kind, payload) = typeTag(type) else { return nil }
        switch kind {
        case "Bool": return reader.readBool()
        case "String": return reader.readString()
        case "Bytes": return reader.readBuffer()?.base64EncodedString()
        case "Numeric":
            guard let name = payload as? String else { return nil }
            switch name {
            case "Int8": return reader.readInt8()
            case "Int16": return reader.readInt16()
            case "Int32": return reader.readInt32()
            case "Int64": return reader.readInt64()
            case "UInt8": return reader.readUint8()
            case "UInt16": return reader.readUint16()
            case "UInt32": return reader.readUint32()
            case "UInt64": return reader.readUint64()
            case "Float32": return reader.readFloat32()
            case "Float64": return reader.readFloat64()
            default: return nil
            }
        case "Enum":
            guard let name = payload as? String,
                  let ordinal = reader.readInt32(),
                  let cases = enumCases[name], cases.indices.contains(Int(ordinal))
            else { return nil }
            return cases[Int(ordinal)]
        case "Struct":
            guard let fields = (payload as? [String: Any])?["fields"] as? [[Any]] else { return nil }
            var values: [String: Any] = [:]
            for field in fields where field.count == 2 {
                guard let name = field[0] as? String,
                      let value = read(type: field[1], from: reader, enumCases: enumCases)
                else { return nil }
                values[name] = value
            }
            return values
        case "Array", "Set":
            guard let count = reader.readCount(), count >= 0, count <= 1_000_000 else { return nil }
            var values: [Any] = []
            values.reserveCapacity(count)
            for _ in 0..<count {
                guard let value = read(type: payload as Any, from: reader, enumCases: enumCases) else { return nil }
                values.append(value)
            }
            if kind == "Set" { return Set(values.map(NexaDevHashableValue.init)) }
            return values
        case "Map":
            guard let types = payload as? [Any], types.count == 2,
                  let count = reader.readCount(), count >= 0, count <= 1_000_000
            else { return nil }
            var values: [AnyHashable: Any] = [:]
            values.reserveCapacity(count)
            for _ in 0..<count {
                guard let key = read(type: types[0], from: reader, enumCases: enumCases),
                      let hashableKey = hashable(key),
                      let value = read(type: types[1], from: reader, enumCases: enumCases)
                else { return nil }
                values[hashableKey] = value
            }
            return values
        case "Pair":
            guard let types = payload as? [Any], types.count == 2,
                  let first = read(type: types[0], from: reader, enumCases: enumCases),
                  let second = read(type: types[1], from: reader, enumCases: enumCases)
            else { return nil }
            return (first, second)
        case "Triple":
            guard let types = payload as? [Any], types.count == 3,
                  let first = read(type: types[0], from: reader, enumCases: enumCases),
                  let second = read(type: types[1], from: reader, enumCases: enumCases),
                  let third = read(type: types[2], from: reader, enumCases: enumCases)
            else { return nil }
            return (first, second, third)
        default:
            return nil
        }
    }

    private static func typeTag(_ raw: Any) -> (String, Any?)? {
        if let raw = raw as? String { return (raw, nil) }
        guard let fields = raw as? [String: Any] else { return nil }
        if let wrappedType = fields["ty"] { return typeTag(wrappedType) }
        guard let (name, payload) = fields.first else { return nil }
        return (name, payload)
    }

    private static func numberValue(_ raw: Any) -> NSNumber? {
        if let value = raw as? NSNumber { return value }
        if let value = raw as? NexaDevHashableValue { return numberValue(value.value) }
        guard let value = raw as? String else { return nil }
        return NSNumber(value: Double(value) ?? 0)
    }

    private static func hashable(_ raw: Any) -> AnyHashable? {
        if let value = raw as? NexaDevHashableValue { return AnyHashable(value) }
        if let value = raw as? String { return AnyHashable(value) }
        if let value = raw as? NSNumber { return AnyHashable(value) }
        if let value = raw as? Int8 { return AnyHashable(value) }
        if let value = raw as? Int16 { return AnyHashable(value) }
        if let value = raw as? Int32 { return AnyHashable(value) }
        if let value = raw as? Int64 { return AnyHashable(value) }
        if let value = raw as? UInt8 { return AnyHashable(value) }
        if let value = raw as? UInt16 { return AnyHashable(value) }
        if let value = raw as? UInt32 { return AnyHashable(value) }
        if let value = raw as? UInt64 { return AnyHashable(value) }
        return nil
    }
}
