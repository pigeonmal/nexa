private class ReferenceGeometry {
    val updates = kotlinx.coroutines.flow.MutableStateFlow(NexaMarqueeMetrics())
    fun measure(content: Int, viewport: Int, cross: Int) {
        updates.value = updates.value.copy(content = content, viewport = viewport, cross = cross)
    }
    fun visibility(visible: Boolean) { updates.value = updates.value.copy(visible = visible) }
}

private object Check {
    @JvmStatic fun main(args: Array<String>) {
        val optimized = NexaMarqueeGeometry()
        val reference = ReferenceGeometry()
        repeat(50000) {
            optimized.measure(300, 120, 44); optimized.visibility(true)
            reference.measure(300, 120, 44); reference.visibility(true)
        }
        val identity = optimized.updates.value
        optimized.measure(300, 120, 44); optimized.visibility(true)
        check(identity === optimized.updates.value)
        optimized.measure(320, 120, 50)
        check(optimized.updates.value.content == 320 && optimized.updates.value.cross == 50)
        optimized.visibility(false)
        check(!optimized.updates.value.visible)
        optimized.measure(300, 120, 44); optimized.visibility(true)
        fun bytes(): Long = android.os.Debug.getRuntimeStat("art.gc.bytes-allocated").toLong()
        val oldSamples = org.json.JSONArray()
        val newSamples = org.json.JSONArray()
        val controlSamples = org.json.JSONArray()
        repeat(7) {
            var before = bytes()
            repeat(20000) { reference.measure(300, 120, 44); reference.visibility(true) }
            oldSamples.put(bytes() - before)
            before = bytes()
            repeat(20000) { optimized.measure(300, 120, 44); optimized.visibility(true) }
            newSamples.put(bytes() - before)
            before = bytes()
            controlSamples.put(bytes() - before)
        }
        println(org.json.JSONObject().put("referenceAllocatedBytes", oldSamples)
            .put("optimizedAllocatedBytes", newSamples).put("counterOverheadBytes", controlSamples)
            .put("unchangedPairsPerSample", 20000).put("apiLevel", android.os.Build.VERSION.SDK_INT)
            .put("scope", "ART cumulative allocation statistics are approximate; isolated native helper process"))
    }
}
