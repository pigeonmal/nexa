import SwiftUI

@MainActor
public final class DevProbePlugin {
    public static let shared = DevProbePlugin()

    private init() {}

    public func increment(_ value: Int32) -> Int32 {
        value + 1
    }

    public func fail() async throws(ProbeError) -> ProbeValue {
        throw .rejected(value: ProbeValue(label: "service", score: 47), reason: "service-probe")
    }
}

@MainActor
public final class ProbeImpl: ProbeSpec {
    public var value = ProbeValue(label: "initial", score: 1)
    public var onChanged: ((ProbeValue) -> Void)?

    public init() {}

    public func updated() -> ProbeValue {
        ProbeValue(label: "updated", score: 42)
    }

    public func numericMap() -> [Int32: Int32] {
        [1: 41, 2: 42]
    }

    public func replace(_ value: ProbeValue) {
        self.value = value
    }

    public func emit() {
        onChanged?(value)
    }

    public func echo<T>(
        _ value: T,
        _ encode: (T, NexaValueWriter) -> Void,
        _ decode: (NexaValueReader) -> T?
    ) -> T {
        value
    }

    public func hasValue<T>(
        _ value: T?,
        _ encode: (T?, NexaValueWriter) -> Void
    ) -> Bool {
        value != nil
    }

    public func echoOptional<T>(
        _ value: T?,
        _ encode: (T?, NexaValueWriter) -> Void,
        _ decode: (NexaValueReader) -> T?
    ) -> T? {
        value
    }

    public func readNullable<T>(_ key: String, _ decode: (NexaValueReader) -> T?) -> T? {
        precondition(!key.isEmpty)
        let writer = NexaValueWriter()
        writer.writeBool(false)
        guard let value = decode(NexaValueReader(writer.data)) else {
            fatalError("nullable generic codec rejected a valid null value")
        }
        return value
    }

    public func fail() async throws(ProbeError) -> ProbeValue {
        throw .rejected(value: value, reason: "runtime-probe")
    }

    public func dispose() {
        onChanged = nil
    }
}

@MainActor
public final class ProbeByteSetImpl: ProbeByteSetSpec {
    public var values: Set<Data> = [Data([0x4E, 0x58]), Data([0x4E, 0x58])]
    public var onChanged: ((Set<Data>) -> Void)?

    public init() {}

    public func replace(_ values: Set<Data>) -> Set<Data> {
        self.values = values
        return values
    }

    public func emit() {
        onChanged?(values)
    }
}

public struct ProbeCardImpl: View {
    private let value: ProbeValue
    private let onSelected: ((ProbeValue) -> Void)?
    @State private var didSendSelection = false

    public init(value: ProbeValue, onSelected: ((ProbeValue) -> Void)?) {
        self.value = value
        self.onSelected = onSelected
    }

    public var body: some View {
        Text(value.label)
            .onAppear {
                guard !didSendSelection else { return }
                didSendSelection = true
                onSelected?(value)
            }
    }
}

public struct ProbeByteSetCardImpl: View {
    private let values: Set<Data>
    private let onSelected: ((Set<Data>) -> Void)?
    @State private var didSendSelection = false

    public init(values: Set<Data>, onSelected: ((Set<Data>) -> Void)?) {
        self.values = values
        self.onSelected = onSelected
    }

    public var body: some View {
        Text("Byte values: \(values.count)")
            .onAppear {
                guard !didSendSelection else { return }
                didSendSelection = true
                onSelected?(values)
            }
    }
}
