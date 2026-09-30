package dev.nexa.runtimeprobe

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect

public class ProbeImpl : ProbeSpec {
    override var value: ProbeValue = ProbeValue("initial", 1)
    override var onChanged: ((ProbeValue) -> Unit)? = null

    override fun updated(): ProbeValue = ProbeValue("updated", 42)

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

    override suspend fun fail(): ProbeValue {
        throw ProbeError.rejected(value, "runtime-probe")
    }

    override fun dispose() {
        onChanged = null
    }
}

@Composable
public fun ProbeCardImpl(value: ProbeValue, onSelected: ((ProbeValue) -> Unit)?) {
    LaunchedEffect(value) {
        onSelected?.invoke(value)
    }
    Text(value.label)
}
