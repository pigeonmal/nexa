package __NEXA_PACKAGE__

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Behavioural tests for the DevRuntime patch engine.
 *
 * These run on the JVM with a real `org.json` implementation so pointer
 * parsing, conflict detection, copy-on-write preparation and the classified
 * commit paths are exercised without an emulator.
 */
class NexaDevPatchTest {
    @Test
    fun optionalStringTreatsJsonNullAsAbsent() {
        val fields = JSONObject().put("focused", JSONObject.NULL)

        assertNull(fields.nexaDevOptionalString("focused"))
        assertNull(JSONObject().nexaDevOptionalString("focused"))
        assertEquals("emailFocused", JSONObject().put("focused", "emailFocused").nexaDevOptionalString("focused"))
    }

    private fun module(
        body: String = "[]",
        screens: String = "[]",
        states: String = "[]",
        functions: String = "[]",
        components: String = "[]",
    ): JSONObject = JSONObject()
        .put("app_name", "PatchTest")
        .put("body", JSONArray(body))
        .put("screens", JSONArray(screens))
        .put("states", JSONArray(states))
        .put("functions", JSONArray(functions))
        .put("components", JSONArray(components))

    private fun operations(vararg values: String): JSONArray = JSONArray().apply {
        values.forEach { put(JSONObject(it)) }
    }

    @Test
    fun nestedSetBuildsCandidateWithoutMutatingBase() {
        val base = module(body = """[{"Text":{"text":"before"}}]""")
        val before = base.toString()
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/0/Text/text","value":"after"}"""),
        )
        assertTrue(prepared != null)
        prepared!!
        assertEquals("before", base.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("text"))
        assertEquals("after", prepared.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("text"))
        assertEquals(before, base.toString())
        assertNotSame(base, prepared.candidate)
        assertSame(base, prepared.baseModule)
    }

    @Test
    fun unchangedSiblingsShareTheOriginalBranch() {
        val base = module(
            body = """[{"Text":{"text":"a"}},{"Text":{"text":"b"}}]""",
        )
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/1/Text/text","value":"c"}"""),
        )!!
        val originalRows = base.getJSONArray("body")
        val candidateRows = prepared.candidate.getJSONArray("body")
        // The patched row is copied; the untouched row keeps structural sharing.
        assertSame(originalRows.get(0), candidateRows.get(0))
        assertNotSame(originalRows.get(1), candidateRows.get(1))
    }

    @Test
    fun invalidPointerIsRejected() {
        val base = module()
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"body/0","value":1}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"/body/~2","value":1}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"/body/~","value":1}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"/body//text","value":1}""")))
    }

    @Test
    fun rootReplacementWithNonObjectIsRejected() {
        val base = module()
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"","value":1}""")))
    }

    @Test
    fun rootRemovalIsRejected() {
        val base = module()
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"remove","path":""}""")))
    }

    @Test
    fun batchedRootReplacementIsRejected() {
        val base = module()
        val replacement = module(body = """[{"Text":{"text":"replaced"}}]""")
        assertNull(
            NexaDevJsonPatch.prepare(
                base,
                operations(
                    """{"op":"set","path":"","value":$replacement}""",
                    """{"op":"set","path":"/version","value":2}"""
                )
            )
        )
    }

    @Test
    fun escapedPathSegmentsAreDecoded() {
        val base = module(body = """[{"Text":{"~key":"x"}}]""")
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/0/Text/~0key","value":"y"}"""),
        )!!
        assertEquals(
            "y",
            prepared.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("~key"),
        )
    }

    @Test
    fun duplicateAndAncestorPathsAreRejected() {
        val base = module(body = """[{"Text":{"text":"x"}}]""")
        assertNull(
            NexaDevJsonPatch.prepare(
                base,
                operations(
                    """{"op":"set","path":"/body/0/Text/text","value":"a"}""",
                    """{"op":"set","path":"/body/0/Text/text","value":"b"}""",
                ),
            ),
        )
        assertNull(
            NexaDevJsonPatch.prepare(
                base,
                operations(
                    """{"op":"remove","path":"/body/0"}""",
                    """{"op":"set","path":"/body/0/Text","value":{}}""",
                ),
            ),
        )
    }

    @Test
    fun missingAndOutOfRangeTargetsAreRejected() {
        val base = module(body = """[{"Text":{"text":"x"}}]""")
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"remove","path":"/missing"}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"/body/4/Text/text","value":"a"}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"remove","path":"/body/0"}""")))
    }

    @Test
    fun candidateRootMustKeepTheModuleShape() {
        val base = module()
        // Setting a whole required section to a non-array breaks the shape.
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"set","path":"/body","value":"nope"}""")))
    }

    @Test
    fun presentationOnlyClassificationRecognizesNestedPresentationValues() {
        val base = module(body = """[{"Text":{"text":"x","font_size":12}}]""")
        val presentation = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/0/Text/font_size","value":18}"""),
        )!!
        assertTrue(presentation.presentationOnly)
        assertFalse(presentation.functionBodiesOnly)
        assertNull(presentation.screenIndices)

        val staticColor = NexaDevJsonPatch.prepare(
            module(body = """[{"Column":{"style":{"background":{"Static":{"red":10,"green":20,"blue":30,"alpha":255}}}}}]"""),
            operations("""{"op":"set","path":"/body/0/Column/style/background/Static/red","value":240}"""),
        )!!
        assertTrue(staticColor.presentationOnly)

        val textValue = NexaDevJsonPatch.prepare(
            module(body = """[{"Text":{"value":{"String":"Before"}}}]"""),
            operations("""{"op":"set","path":"/body/0/Text/value/String","value":"After"}"""),
        )!!
        assertTrue(textValue.presentationOnly)

        val structural = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/0/Text","value":{}}"""),
        )!!
        assertFalse(structural.presentationOnly)
    }

    @Test
    fun functionBodyOnlyClassificationIsDetected() {
        val base = module(
            functions = """[{"name":"increment","body":[],"body_actions":[]}]""",
        )
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/functions/0/body","value":[]}"""),
        )!!
        assertTrue(prepared.functionBodiesOnly)
        assertFalse(prepared.presentationOnly)

        val actions = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/functions/0/body_actions","value":[]}"""),
        )!!
        assertTrue(actions.functionBodiesOnly)
    }

    @Test
    fun screenScopedClassificationCollectsIndices() {
        val base = module(
            screens = """[{"name":"Home","body":[]},{"name":"Settings","body":[]}]""",
        )
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations(
                """{"op":"set","path":"/screens/0/body","value":[]}""",
                """{"op":"set","path":"/screens/1/name","value":"Preferences"}""",
            ),
        )!!
        assertEquals(setOf(0, 1), prepared.screenIndices)
        assertFalse(prepared.presentationOnly)
        assertFalse(prepared.functionBodiesOnly)
    }

    @Test
    fun mixedScopeFallsBackToFullInstall() {
        val base = module(screens = """[{"name":"Home","body":[]}]""")
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations(
                """{"op":"set","path":"/screens/0/body","value":[]}""",
                """{"op":"set","path":"/app_name","value":"Renamed"}""",
            ),
        )!!
        assertNull(prepared.screenIndices)
        assertFalse(prepared.presentationOnly)
        assertFalse(prepared.functionBodiesOnly)
    }

    @Test
    fun nestedSetOnNewPropertyBuildsCandidateWithoutMutatingBase() {
        val base = module(body = """[{"Text":{"text":"before"}}]""")
        val before = base.toString()
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/0/Text/font_size","value":18}"""),
        )
        assertTrue(prepared != null)
        prepared!!
        assertFalse(base.getJSONArray("body").getJSONObject(0).getJSONObject("Text").has("font_size"))
        assertEquals(18, prepared.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getInt("font_size"))
        assertEquals("before", prepared.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("text"))
        assertEquals(before, base.toString())
        assertTrue(prepared.presentationOnly)
    }

    @Test
    fun arrayAppendAndRemoveOperations() {
        val base = module(body = """[{"Text":{"text":"row0"}}]""")
        val appended = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"/body/-","value":{"Text":{"text":"row1"}}}"""),
        )!!
        assertEquals(2, appended.candidate.getJSONArray("body").length())
        assertEquals("row1", appended.candidate.getJSONArray("body").getJSONObject(1).getJSONObject("Text").getString("text"))

        val removed = NexaDevJsonPatch.prepare(
            appended.candidate,
            operations("""{"op":"remove","path":"/body/0"}"""),
        )!!
        assertEquals(1, removed.candidate.getJSONArray("body").length())
        assertEquals("row1", removed.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("text"))
    }

    @Test
    fun rootReplacementWithValidModule() {
        val base = module(body = """[{"Text":{"text":"initial"}}]""")
        val replacement = module(body = """[{"Text":{"text":"replaced"}}]""")
        val prepared = NexaDevJsonPatch.prepare(
            base,
            operations("""{"op":"set","path":"","value":$replacement}"""),
        )!!
        assertEquals("replaced", prepared.candidate.getJSONArray("body").getJSONObject(0).getJSONObject("Text").getString("text"))
    }

    @Test
    fun removeNonexistentPropertyIsRejected() {
        val base = module(body = """[{"Text":{"text":"x"}}]""")
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"remove","path":"/body/0/Text/nonexistent"}""")))
    }

    @Test
    fun unsupportedOperationKindsAreRejected() {
        val base = module()
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"move","path":"/body/0","value":1}""")))
        assertNull(NexaDevJsonPatch.prepare(base, operations("""{"op":"add","path":"/body/0","value":1}""")))
    }

    @Test
    fun emptyOperationListIsRejected() {
        assertNull(NexaDevJsonPatch.prepare(module(), JSONArray()))
    }
}
