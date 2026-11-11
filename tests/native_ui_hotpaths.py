#!/usr/bin/env python3
"""Execute the exact emitted NumberFlow and scroll-observation helpers with the native toolchains.

No UI stubs or screenshot capture. Gradle's cached Kotlin compiler is optional;
--require-all makes absence of either toolchain an error. --before-swift accepts
an earlier number_flow.swift to benchmark unchanged-value parent refresh work.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(args):
    result = subprocess.run([str(arg) for arg in args], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(f"Command failed: {args[0]}\n{result.stdout}\n{result.stderr}")
    return result.stdout.strip()


SWIFT_CHECK = r'''
@main struct NativeCheck {
    @MainActor static func main() {
        let cache = NexaNumberFlowCache()
        var notifications = 0
        let subscription = cache.objectWillChange.sink { notifications += 1 }
        func key(_ value: Double, _ format: String = "Number", _ currency: String? = nil,
                 _ minimum: Int? = nil, _ maximum: Int? = nil, _ locale: String = "en_US") -> NexaNumberFlowCache.Key {
            .init(value: value, format: format, currencyCode: currency,
                  minimumFractionDigits: minimum, maximumFractionDigits: maximum,
                  locale: Locale(identifier: locale))
        }
        for locale in ["en_US", "fr_FR", "de_DE", "ar_EG"] {
            for value in [-1000.25, -0.01, 0, 0.125, 9.999, 99.995, 10000.5] {
                let configuration = key(value, "Number", nil, 2, 2, locale)
                let presentation = cache.presentation(for: configuration)
                let expected = value.formatted(FloatingPointFormatStyle<Double>.number.locale(configuration.locale).precision(.fractionLength(2...2)))
                precondition(presentation.formatted == expected)
                precondition(presentation.tokens.map(\.character) == Array(expected))
                let positions = presentation.tokens.compactMap(\.digitPosition)
                precondition(positions == Array((0..<positions.count).reversed()))
                let second = cache.presentation(for: configuration)
                presentation.tokens.withUnsafeBufferPointer { first in
                    second.tokens.withUnsafeBufferPointer { precondition(first.baseAddress == $0.baseAddress) }
                }
                precondition(cache.digitGlyphs(locale: configuration.locale).count == 10)
            }
        }
        let rounded = cache.presentation(for: key(1.001, "Number", nil, 2, 2))
        let sameDisplay = cache.presentation(for: key(1.002, "Number", nil, 2, 2))
        rounded.tokens.withUnsafeBufferPointer { first in
            sameDisplay.tokens.withUnsafeBufferPointer { precondition(first.baseAddress == $0.baseAddress) }
        }
        precondition(cache.presentation(for: key(0.25, "Percent")).formatted == "25%")
        let eur = cache.presentation(for: key(12.5, "Currency", "EUR", 2, 2)).formatted
        let usd = cache.presentation(for: key(12.5, "Currency", "USD", 2, 2)).formatted
        precondition(eur != usd)
        precondition(cache.presentation(for: key(12.5, "Number", nil, 0, 0)).formatted != cache.presentation(for: key(12.5, "Number", nil, 2, 2)).formatted)
        precondition(cache.presentation(for: key(12000, "Compact")).formatted == 12000.0.formatted(.number.notation(.compactName).locale(Locale(identifier: "en_US"))))
        cache.commit(100)
        precondition(!cache.direction(for: 99))
        cache.commit(99)
        // Unrelated parent updates must preserve the decreasing transition.
        for _ in 0..<1000 { precondition(!cache.direction(for: 99)); cache.commit(99) }
        precondition(cache.direction(for: 101))
        cache.commit(101)
        precondition(cache.direction(for: 101))
        precondition(notifications == 0, "Memoization must not invalidate SwiftUI")
        withExtendedLifetime(subscription) {}
        BENCHMARK
        print("native Swift NumberFlow: formatting, keys, token identity, direction, zero publications passed")
    }
}
'''

KOTLIN_CHECK = r'''
fun main() {
    val history = NexaNumberFlowHistory(100.0)
    check(!history.direction(99.0))
    history.commit(99.0)
    repeat(1000) { check(!history.direction(99.0)); history.commit(99.0) }
    check(history.direction(101.0))
    history.commit(101.0)
    check(history.direction(101.0))
    check(!history.direction(-1.0))
    history.commit(-1.0)
    check(!history.direction(-2.0))
    history.commit(-2.0)
    check(history.direction(0.0))
    checkScrollObservation()
    checkMarqueeGeometry()
    checkNetworkByteArrayBuilder()
    println("native Kotlin NumberFlow: committed direction survives unchanged renders passed")
}

private fun checkNetworkByteArrayBuilder() {
    val bytes = ByteArray(2048) { (it % 251).toByte() }
    val warmup = NexaByteArrayBuilder(bytes.size)
    warmup.write(bytes, 0, bytes.size)
    warmup.take()

    val exact = NexaByteArrayBuilder(bytes.size)
    exact.write(bytes, 0, bytes.size)
    val bean = java.lang.management.ManagementFactory.getThreadMXBean()
        as? com.sun.management.ThreadMXBean
    if (bean != null && bean.isThreadAllocatedMemorySupported) {
        bean.isThreadAllocatedMemoryEnabled = true
        val thread = Thread.currentThread().id
        val before = bean.getThreadAllocatedBytes(thread)
        val exactBody = exact.take()
        val allocated = bean.getThreadAllocatedBytes(thread) - before
        check(allocated == 0L) { "exact-sized response transfer allocated $allocated bytes" }
        check(exactBody.contentEquals(bytes))
    } else {
        check(exact.take().contentEquals(bytes))
    }

    val growing = NexaByteArrayBuilder(1024)
    val first = ByteArray(300) { 1 }
    val second = ByteArray(400) { 2 }
    growing.write(first, 0, first.size)
    growing.write(second, 0, second.size)
    val combined = growing.take()
    check(combined.size == 700)
    check(combined.copyOfRange(0, 300).contentEquals(first))
    check(combined.copyOfRange(300, 700).contentEquals(second))
    check(growing.size == 0)

    val bounded = NexaByteArrayBuilder(8)
    bounded.write(first, 0, 8)
    var rejected = false
    try { bounded.write(first, 8, 1) } catch (_: IllegalArgumentException) { rejected = true }
    check(rejected && bounded.size == 8)
    check(bounded.take().contentEquals(first.copyOfRange(0, 8)))
    println("network response builder: exact-capacity ownership, growth, and size bounds passed")
}

private data class Item(val index: Int, val offset: Int, val size: Int)

// Original AOT behavior is the reference: prefetched items do not count;
// pinned headers obscure rows, with a conservative visible fallback.
private fun reference(items: List<Item>, fallback: Int, headerOffset: Int, reverse: Boolean, start: Int, end: Int): Int {
    val visible = items.filter { it.offset + it.size > start && it.offset < end }
    val header = if (headerOffset > 0) visible.firstOrNull { it.index == 0 } else null
    var leading = -1
    for (item in visible) {
        if (item.index < headerOffset) continue
        if (header != null && item.offset < header.offset + header.size && item.offset + item.size > header.offset) continue
        if (leading < 0 || (reverse && item.index > leading) || (!reverse && item.index < leading)) leading = item.index
    }
    return if (leading >= 0) leading else if (reverse) visible.maxOfOrNull { it.index } ?: fallback
        else visible.firstOrNull { it.index >= headerOffset }?.index ?: fallback
}

private fun observed(items: List<Item>, fallback: Int, headerOffset: Int, reverse: Boolean, start: Int, end: Int): Int =
    nexaLeadingVisibleDataItemIndex(items, fallback, headerOffset, reverse, start, end,
        itemIndex = { it.index }, itemOffset = { it.offset }, itemSize = { it.size })

private fun checkScrollObservation() {
    val random = kotlin.random.Random(6418)
    repeat(1000) {
        val items = (0 until random.nextInt(0, 40)).map { Item(it, random.nextInt(-500, 1000), random.nextInt(0, 200)) }.shuffled(random)
        val start = random.nextInt(-100, 200)
        val end = start + random.nextInt(0, 800)
        for (header in 0..1) for (reverse in listOf(false, true)) {
            val expected = reference(items, 17, header, reverse, start, end)
            check(observed(items, 17, header, reverse, start, end) == expected)
        }
    }
    val items = (0 until 40).map { Item(it, it * 56 - 112, 56) }
    var checksum = 0
    repeat(20000) {
        checksum += observed(items, 0, 1, false, 0, 800)
        checksum += reference(items, 0, 1, false, 0, 800)
    }
    val bean = java.lang.management.ManagementFactory.getThreadMXBean() as? com.sun.management.ThreadMXBean
    if (bean != null && bean.isThreadAllocatedMemorySupported) {
        bean.isThreadAllocatedMemoryEnabled = true
        val thread = Thread.currentThread().id
        var before = bean.getThreadAllocatedBytes(thread)
        repeat(20000) { checksum += reference(items, 0, 1, false, 0, 800) }
        val referenceBytes = bean.getThreadAllocatedBytes(thread) - before
        before = bean.getThreadAllocatedBytes(thread)
        repeat(20000) { checksum += observed(items, 0, 1, false, 0, 800) }
        val optimizedBytes = bean.getThreadAllocatedBytes(thread) - before
        println("scroll allocations: reference=$referenceBytes optimized=$optimizedBytes iterations=20000 checksum=$checksum")
    }
    println("native Kotlin scroll observation: 4000 reference comparisons passed")
}


private class ReferenceMarqueeGeometry {
    val updates = kotlinx.coroutines.flow.MutableStateFlow(NexaMarqueeMetrics())
    fun measure(content: Int, viewport: Int, cross: Int) {
        updates.value = updates.value.copy(content = content, viewport = viewport, cross = cross)
    }
    fun visibility(visible: Boolean) { updates.value = updates.value.copy(visible = visible) }
}

private fun checkMarqueeGeometry() {
    val geometry = NexaMarqueeGeometry()
    geometry.measure(300, 120, 44)
    geometry.visibility(true)
    val snapshot = geometry.updates.value
    repeat(100) { geometry.measure(300, 120, 44); geometry.visibility(true) }
    check(geometry.updates.value === snapshot)
    geometry.measure(400, 140, 50)
    check(geometry.updates.value.content == 400 && geometry.updates.value.viewport == 140 && geometry.updates.value.cross == 50)
    geometry.visibility(false)
    check(!geometry.updates.value.visible)
    val reference = ReferenceMarqueeGeometry()
    repeat(20000) {
        geometry.measure(300, 120, 44); geometry.visibility(true)
        reference.measure(300, 120, 44); reference.visibility(true)
    }
    val bean = java.lang.management.ManagementFactory.getThreadMXBean() as? com.sun.management.ThreadMXBean
    if (bean != null && bean.isThreadAllocatedMemorySupported) {
        bean.isThreadAllocatedMemoryEnabled = true
        val thread = Thread.currentThread().id
        var before = bean.getThreadAllocatedBytes(thread)
        repeat(20000) { reference.measure(300, 120, 44); reference.visibility(true) }
        val referenceBytes = bean.getThreadAllocatedBytes(thread) - before
        before = bean.getThreadAllocatedBytes(thread)
        repeat(20000) { geometry.measure(300, 120, 44); geometry.visibility(true) }
        val optimizedBytes = bean.getThreadAllocatedBytes(thread) - before
        println("marquee geometry allocations: reference=$referenceBytes optimized=$optimizedBytes unchangedPairs=20000")
    }
    println("native Kotlin marquee: unchanged snapshots, changed metrics, and visibility passed")
}

'''


def kotlin_jars():
    root = Path(os.environ.get("GRADLE_USER_HOME", Path.home() / ".gradle")) / "caches/modules-2/files-2.1"
    def jar(group, name, version=None):
        matches = sorted((root / group / name).glob(f"{version or '*'}/*/{name}-*.jar"))
        return matches[-1] if matches else None
    compiler = jar("org.jetbrains.kotlin", "kotlin-compiler-embeddable")
    if compiler is None:
        return None
    version = compiler.parent.parent.name
    dependencies = [compiler]
    for group, name, required_version in [
        ("org.jetbrains.kotlin", "kotlin-stdlib", version),
        ("org.jetbrains.kotlin", "kotlin-build-tools-api", version),
        ("org.jetbrains.kotlin", "kotlin-script-runtime", version),
        ("org.jetbrains.kotlin", "kotlin-reflect", None),
        ("org.jetbrains.kotlin", "kotlin-daemon-embeddable", version),
        ("org.jetbrains.kotlinx", "kotlinx-coroutines-core-jvm", None),
        ("org.jetbrains", "annotations", None),
    ]:
        dependency = jar(group, name, required_version)
        if dependency is None:
            return None
        dependencies.append(dependency)
    return dependencies


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-all", action="store_true")
    parser.add_argument("--before-swift", type=Path)
    args = parser.parse_args()
    results = {}
    with tempfile.TemporaryDirectory(prefix="nexa-native-number-flow-") as directory:
        scratch = Path(directory)
        swiftc = shutil.which("swiftc")
        if swiftc and os.uname().sysname == "Darwin":
            source = (ROOT / "crates/nexa-backend-swift/src/generator/components/number_flow.swift").read_text().split("struct NexaNumberFlowPrimitive: View {")[0]
            benchmark = ""
            if args.before_swift:
                before = args.before_swift.read_text()
                methods = before.split("    private var digitGlyphs: [String] {")[1].split("    var body: some View {")[0]
                source += '''\n@MainActor private final class BeforeNumberFlow {
    let value = 999.99
    let format = "Currency"
    let currencyCode: String? = "EUR"
    let minimumFractionDigits: Int? = nil
    let maximumFractionDigits: Int? = 2
    let locale = Locale(identifier: "en_US")
    let digitFormatter = NumberFormatter()
    var cachedDigitGlyphs: [String] = []
    var digitGlyphsLocaleIdentifier = ""
    var digitGlyphs: [String] {''' + methods.replace("private ", "") + "}\n"
                benchmark = r'''
        let before = BeforeNumberFlow()
        before.refreshDigitGlyphs()
        let configuration = key(999.99, "Currency", "EUR", nil, 2)
        var checksums = [0, 0]
        var oldSamples: [Double] = [], newSamples: [Double] = []
        for _ in 0..<9 {
            var start = ContinuousClock.now
            for _ in 0..<5000 {
                let formatted = before.formattedValue
                checksums[0] += before.tokens(for: formatted).count + before.digitGlyphs.count
            }
            oldSamples.append(Double(start.duration(to: .now).components.attoseconds) / 1e15)
            start = .now
            for _ in 0..<5000 {
                checksums[1] += cache.presentation(for: configuration).tokens.count + cache.digitGlyphs(locale: configuration.locale).count
            }
            newSamples.append(Double(start.duration(to: .now).components.attoseconds) / 1e15)
        }
        precondition(checksums[0] == checksums[1])
        let output: [String: Any] = ["iterations": 5000, "beforeMs": oldSamples, "afterMs": newSamples, "checksum": checksums[0]]
        print(String(data: try! JSONSerialization.data(withJSONObject: output, options: [.sortedKeys]), encoding: .utf8)!)
'''
            path = scratch / "Check.swift"
            path.write_text("import Foundation\nimport Combine\n" + source + SWIFT_CHECK.replace("BENCHMARK", benchmark))
            binary = scratch / "swift-check"
            run([swiftc, "-O", "-swift-version", "6", "-parse-as-library", path, "-o", binary])
            results["swift"] = run([binary])
        else:
            results["swift"] = "unavailable"
        dependencies = kotlin_jars()
        java = shutil.which("java")
        if java and dependencies:
            source = (ROOT / "crates/nexa-backend-kotlin/src/generator/components/number_flow.kt").read_text().split("@androidx.compose.runtime.Composable")[0]
            path = scratch / "Check.kt"
            source += (ROOT / "crates/nexa-backend-kotlin/src/generator/components/list_observation.kt").read_text()
            source += (ROOT / "crates/nexa-backend-kotlin/src/generator/components/marquee.kt").read_text().split("@androidx.compose.runtime.Composable", 1)[0]
            runtime_helpers = (ROOT / "crates/nexa-backend-kotlin/src/generator/api/network_runtime.kt").read_text()
            path.write_text("import java.io.OutputStream\n" + source + runtime_helpers + KOTLIN_CHECK)
            classpath = os.pathsep.join(str(path) for path in dependencies)
            output = scratch / "kotlin-classes"
            run([java, "-cp", classpath, "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler", "-no-stdlib", "-no-reflect", "-classpath", classpath, path, "-d", output])
            results["kotlin"] = run([java, "-cp", str(output) + os.pathsep + classpath, "CheckKt"])
        else:
            results["kotlin"] = "unavailable"
    print(json.dumps(results, indent=2))
    if args.require_all and "unavailable" in results.values():
        raise SystemExit("Both native toolchains are required")


if __name__ == "__main__":
    main()
