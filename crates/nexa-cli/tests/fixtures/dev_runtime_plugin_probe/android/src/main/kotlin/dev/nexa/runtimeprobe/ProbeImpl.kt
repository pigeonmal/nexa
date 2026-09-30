package dev.nexa.runtimeprobe

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect

public class DevProbePlugin private constructor() {
    public companion object {
        public val instance: DevProbePlugin = DevProbePlugin()
    }

    fun increment(value: Int): Int = value + 1

    suspend fun fail(): ProbeValue {
        throw ProbeError.rejected(ProbeValue("service", 47), "service-probe")
    }
}

public class ProbeImpl : ProbeSpec {
    override var value: ProbeValue = ProbeValue("initial", 1)
    override var onChanged: ((ProbeValue) -> Unit)? = null

    override fun updated(): ProbeValue = ProbeValue("updated", 42)

    override fun numericMap(): Map<Int, Int> = mapOf(1 to 41, 2 to 42)

    override fun replace(value: ProbeValue) {
        this.value = value
    }

    override fun emit() {
        onChanged?.invoke(value)
    }

    override fun <T> echo(
        value: T,
        encode0: (T, dev.nexa.core.NexaValueWriter) -> Unit,
        decode1: (dev.nexa.core.NexaValueReader) -> dev.nexa.core.NexaValueReadResult<T>,
    ): T = value

    override fun <T> hasValue(
        value: T?,
        encode0: (T?, dev.nexa.core.NexaValueWriter) -> Unit,
    ): Boolean = value != null

    override fun <T> echoOptional(
        value: T?,
        encode0: (T?, dev.nexa.core.NexaValueWriter) -> Unit,
        decode1: (dev.nexa.core.NexaValueReader) -> dev.nexa.core.NexaValueReadResult<T>,
    ): T? = value

    override fun <T> readNullable(
        key: String,
        decode0: (dev.nexa.core.NexaValueReader) -> dev.nexa.core.NexaValueReadResult<T>,
    ): T? {
        check(key.isNotEmpty())
        val writer = dev.nexa.core.NexaValueWriter()
        writer.writeBool(false)
        val decoded = decode0(dev.nexa.core.NexaValueReader(writer.toByteArray()))
        return when (decoded) {
            is dev.nexa.core.NexaValueReadResult.Value<*> -> {
                @Suppress("UNCHECKED_CAST")
                decoded.value as T?
            }
            dev.nexa.core.NexaValueReadResult.Invalid ->
                error("nullable generic codec rejected a valid null value")
        }
    }

    override suspend fun fail(): ProbeValue {
        throw ProbeError.rejected(value, "runtime-probe")
    }

    override fun dispose() {
        onChanged = null
    }
}

public class ProbeByteSetImpl : ProbeByteSetSpec {
    override var values: Set<ByteArray> = setOf(byteArrayOf(0x4E, 0x58), byteArrayOf(0x4E, 0x58))
    override var onChanged: ((Set<ByteArray>) -> Unit)? = null

    override fun replace(values: Set<ByteArray>): Set<ByteArray> {
        this.values = values
        return values
    }

    override fun emit() {
        onChanged?.invoke(values)
    }
}

@Composable
public fun ProbeCardImpl(value: ProbeValue, onSelected: ((ProbeValue) -> Unit)?) {
    LaunchedEffect(value) {
        onSelected?.invoke(value)
    }
    Text(value.label)
}

@Composable
public fun ProbeByteSetCardImpl(values: Set<ByteArray>, onSelected: ((Set<ByteArray>) -> Unit)?) {
    LaunchedEffect(values) {
        onSelected?.invoke(values)
    }
    Text("Byte values: ${values.size}")
}
