//! Value-codec naming and collection for generic plugin calls.
//!
//! A plugin method may declare value type parameters. Each call site binds
//! them to a concrete type, and the backend emits one writer or reader
//! function per bound type. Everything about that mapping is deterministic:
//! the same type always produces the same function name, and the required
//! functions are collected in one pass so a type's own codec is always
//! emitted before a codec that calls it.

use nexa_ir::walk::{IrVisitor, walk_ir};
use nexa_ir::{NumericType, Type};

/// Which half of a codec a function implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Write,
    Read,
}

/// One generated codec function: the type it handles and the direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codec {
    pub ty: Type,
    pub direction: Direction,
}

/// Name of the generated function that writes or reads `ty`.
///
/// The mangling is built from the language type, not the platform spelling,
/// so both backends agree on it and a platform rename cannot change a
/// collection's name.
pub fn codec_name(ty: &Type, direction: Direction) -> String {
    let prefix = match direction {
        Direction::Write => "nexaWrite",
        Direction::Read => "nexaRead",
    };
    format!("{prefix}{}", mangled(ty, None))
}

/// Numeric spellings used in generated codec names.
pub fn numeric_mangled(numeric: NumericType) -> &'static str {
    match numeric {
        NumericType::Int8 => "int8",
        NumericType::Int16 => "int16",
        NumericType::Int32 => "int32",
        NumericType::Int64 => "int64",
        NumericType::UInt8 => "uint8",
        NumericType::UInt16 => "uint16",
        NumericType::UInt32 => "uint32",
        NumericType::UInt64 => "uint64",
        NumericType::Float32 => "float32",
        NumericType::Float64 => "float64",
    }
}

/// A stable, collision-free mangling of a type: lowercase words joined with
/// `_`, with the parent path carried into nested names.
fn mangled(ty: &Type, parent: Option<&str>) -> String {
    let own = |word: &str| -> String {
        match parent {
            Some(parent) => format!("{parent}_{word}"),
            None => word.to_owned(),
        }
    };
    match ty {
        Type::Bool => own("bool"),
        Type::Numeric(numeric) => own(numeric_mangled(*numeric)),
        Type::String => own("string"),
        Type::Bytes => own("bytes"),
        Type::Enum(name) => own(&format!("enum_{}", lowercased(name))),
        Type::Struct { name, .. } => own(&format!("struct_{}", lowercased(name))),
        Type::Array(element) => {
            let prefix = own("array");
            mangled(element, Some(&prefix))
        }
        Type::Set(element) => {
            let prefix = own("set");
            mangled(element, Some(&prefix))
        }
        Type::Map(key, value) => {
            let prefix = own("map");
            let prefix = mangled(key, Some(&prefix));
            mangled(value, Some(&prefix))
        }
        Type::Pair(first, second) => {
            let prefix = own("pair");
            let prefix = mangled(first, Some(&prefix));
            mangled(second, Some(&prefix))
        }
        Type::Triple(first, second, third) => {
            let prefix = own("triple");
            let prefix = mangled(first, Some(&prefix));
            let prefix = mangled(second, Some(&prefix));
            mangled(third, Some(&prefix))
        }
        // Unreachable for a bound plugin value type: the compiler rejects
        // every other shape before code generation.
        other => own(&format!(
            "unsupported_{}",
            lowercased(&format!("{other:?}"))
        )),
    }
}

fn lowercased(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// Every codec function a module needs, children before parents, one entry
/// per distinct type and direction.
///
/// The walk covers every place an expression can live: node bodies, state
/// initializers, app and screen lifecycle actions, and app function bodies.
/// A codec is emitted because a call uses it, so a type nothing passes to a
/// generic plugin method costs nothing.
pub fn collect(module: &nexa_ir::Module) -> Vec<Codec> {
    let mut collector = CodecCollector::default();
    collector.module(module);
    collector.codecs
}

#[derive(Default)]
struct CodecCollector {
    codecs: Vec<Codec>,
}

impl IrVisitor for CodecCollector {
    fn visit_expr(&mut self, expr: &nexa_ir::Expr) {
        if let nexa_ir::Expr::NativeCall { codecs, .. } = expr {
            for codec in codecs {
                let direction = if codec.decodes {
                    Direction::Read
                } else {
                    Direction::Write
                };
                self.add(&codec.ty, direction);
            }
        }
        nexa_ir::walk::walk_expr_children(expr, self);
    }
}

impl CodecCollector {
    /// Walks every place an expression can live: node bodies, state
    /// initializers, app and screen lifecycle actions, and app function bodies.
    fn module(&mut self, module: &nexa_ir::Module) {
        self.nodes(&module.body);
        for state in &module.states {
            self.visit_expr(&state.initial);
        }
        for function in &module.functions {
            for local in &function.locals {
                self.visit_expr(&local.initial);
            }
            self.visit_expr(&function.body);
        }
        for actions in [
            &module.on_appear,
            &module.on_disappear,
            &module.on_active,
            &module.on_inactive,
            &module.on_background,
        ] {
            self.actions(actions);
        }
        for screen in &module.screens {
            self.nodes(&screen.body);
            for state in &screen.states {
                self.visit_expr(&state.initial);
            }
            self.actions(&screen.on_appear);
            self.actions(&screen.on_disappear);
        }
        for component in &module.components {
            self.nodes(&component.body);
            for state in &component.states {
                self.visit_expr(&state.initial);
            }
        }
    }

    fn actions(&mut self, actions: &Option<Vec<nexa_ir::Action>>) {
        if let Some(actions) = actions {
            nexa_ir::walk::walk_actions(actions, &mut |expr| self.visit_expr(expr));
        }
    }

    fn nodes(&mut self, nodes: &[nexa_ir::Node]) {
        let mut noop = |_: &nexa_ir::Node| {};
        let mut visit = |expr: &nexa_ir::Expr| self.visit_expr(expr);
        walk_ir(nodes, &mut noop, &mut visit);
    }

    /// Records `ty` and every type nested inside it, innermost first, so a
    /// collection's codec is always declared after the codecs it calls.
    fn add(&mut self, ty: &Type, direction: Direction) {
        match ty {
            Type::Array(element) | Type::Set(element) => self.add(element, direction),
            Type::Map(key, value) => {
                self.add(key, direction);
                self.add(value, direction);
            }
            Type::Pair(first, second) => {
                self.add(first, direction);
                self.add(second, direction);
            }
            Type::Triple(first, second, third) => {
                self.add(first, direction);
                self.add(second, direction);
                self.add(third, direction);
            }
            Type::Struct { fields, .. } => {
                for (_, field) in fields {
                    self.add(field, direction);
                }
            }
            _ => {}
        }
        let codec = Codec {
            ty: ty.clone(),
            direction,
        };
        if !self.codecs.contains(&codec) {
            self.codecs.push(codec);
        }
    }
}

// The generated Kotlin core runtime.
//
// A plugin's Kotlin contract and implementation are generated into, and
// written in, the *plugin's* package, so anything they must share with the
// app lives in one fixed core package. Two things are there: the application
// context holder, because a plugin that opens a platform resource needs the
// application and has no other way to reach it, and the value codec, because a
// generic plugin method's contract names it in a signature.
//
// The text lives here rather than in a backend so `nexa plugin check` can
// compile a plugin contract against exactly the runtime the app will generate.

/// The fixed package the generated core runtime lives in.
pub const KOTLIN_CORE_PACKAGE: &str = "dev.nexa.core";

/// The application-context holder, package declaration included.
pub fn kotlin_core_runtime_source() -> String {
    format!(
        "package {KOTLIN_CORE_PACKAGE}\n\n{}",
        CORE_RUNTIME.trim_start()
    )
}

const CORE_RUNTIME: &str = r#"/**
 * The application context, published so generated code and plugins in other
 * packages can reach it.
 *
 * The app binds the context once, from its activity, before any native API
 * runs. Reads are volatile because the binding happens on the main thread and
 * a plugin may read from a worker.
 */
public object NexaRuntimeCore {
    @Volatile private var applicationContext: android.content.Context? = null
    @Volatile private var foregroundActivity: java.lang.ref.WeakReference<android.app.Activity>? = null

    public fun bind(context: android.content.Context) {
        val application = context.applicationContext
        if (applicationContext !== application) applicationContext = application
        var current: android.content.Context? = context
        while (current != null) {
            if (current is android.app.Activity) {
                foregroundActivity = java.lang.ref.WeakReference(current)
                break
            }
            val wrapper = current as? android.content.ContextWrapper ?: break
            if (wrapper.baseContext === current) break
            current = wrapper.baseContext
        }
    }

    public fun context(): android.content.Context = requireNotNull(applicationContext) {
        "NexaRuntime.bind must run before a native API call"
    }

    public fun dismissKeyboard() {
        val activity = foregroundActivity?.get() ?: return
        activity.runOnUiThread {
            val decorView = activity.window.decorView
            val view = activity.currentFocus ?: decorView
            val token = view.windowToken ?: decorView.windowToken ?: return@runOnUiThread
            val manager = activity.getSystemService(android.content.Context.INPUT_METHOD_SERVICE)
                as? android.view.inputmethod.InputMethodManager ?: return@runOnUiThread
            manager.hideSoftInputFromWindow(token, 0)
        }
    }
}
"#;

/// The value-codec runtime, package declaration included.
pub fn kotlin_runtime_source() -> String {
    format!("package {KOTLIN_CORE_PACKAGE}\n\n{}", RUNTIME.trim_start())
}

const RUNTIME: &str = r#"// Plugin value codecs

/** Append-only little-endian writer for a plugin value codec. */
public class NexaValueWriter {
    private var buffer = ByteArray(64)
    private var position = 0

    private fun reserve(count: Int) {
        if (position + count > buffer.size) {
            var capacity = buffer.size * 2
            while (capacity < position + count) {
                capacity *= 2
            }
            buffer = buffer.copyOf(capacity)
        }
    }

    fun writeByte(value: Int) {
        reserve(1)
        buffer[position] = value.toByte()
        position += 1
    }

    fun writeRaw(value: ByteArray) {
        reserve(value.size)
        value.copyInto(buffer, position)
        position += value.size
    }

    fun writeCount(count: Int) {
        writeInt32(count)
    }

    fun writeBool(value: Boolean) {
        writeByte(if (value) 1 else 0)
    }

    fun writeString(value: String) {
        val utf8 = value.toByteArray(Charsets.UTF_8)
        writeCount(utf8.size)
        writeRaw(utf8)
    }

    fun writeBuffer(value: ByteArray) {
        writeCount(value.size)
        writeRaw(value)
    }

    fun writeInt8(value: Byte) {
        writeByte(value.toInt())
    }

    fun writeInt16(value: Short) {
        writeByte(value.toInt())
        writeByte(value.toInt() ushr 8)
    }

    fun writeInt32(value: Int) {
        writeByte(value)
        writeByte(value ushr 8)
        writeByte(value ushr 16)
        writeByte(value ushr 24)
    }

    fun writeInt64(value: Long) {
        writeInt32(value.toInt())
        writeInt32((value ushr 32).toInt())
    }

    fun writeUInt8(value: UByte) {
        writeInt8(value.toByte())
    }

    fun writeUInt16(value: UShort) {
        writeInt16(value.toShort())
    }

    fun writeUInt32(value: UInt) {
        writeInt32(value.toInt())
    }

    fun writeUInt64(value: ULong) {
        writeInt64(value.toLong())
    }

    fun writeFloat32(value: Float) {
        writeInt32(java.lang.Float.floatToRawIntBits(value))
    }

    fun writeFloat64(value: Double) {
        writeInt64(java.lang.Double.doubleToRawLongBits(value))
    }

    fun toByteArray(): ByteArray = buffer.copyOf(position)
}

/**
 * Bounds-checked reader over an encoded value.
 *
 * The payload is wrapped once, so a fixed-width read is a bounds check and a
 * load with no allocation. Only a string or a byte buffer copies, and that copy
 * is the value being returned.
 */
public class NexaValueReader(private val bytes: ByteArray) {
    private val buffer: java.nio.ByteBuffer =
        java.nio.ByteBuffer.wrap(bytes).order(java.nio.ByteOrder.LITTLE_ENDIAN)

    private fun has(count: Int): Boolean =
        count >= 0 && buffer.remaining() >= count

    private fun take(count: Int): ByteArray? {
        if (!has(count)) {
            return null
        }
        val slice = ByteArray(count)
        buffer.get(slice)
        return slice
    }

    fun readCount(): Int? = readInt32()

    fun readBool(): Boolean? {
        val raw = readUInt8()?.toInt() ?: return null
        if (raw > 1) {
            return null
        }
        return raw == 1
    }

    fun readString(): String? {
        val count = readCount() ?: return null
        val raw = take(count) ?: return null
        return raw.toString(Charsets.UTF_8)
    }

    fun readBuffer(): ByteArray? {
        val count = readCount() ?: return null
        return take(count)
    }

    fun readInt8(): Byte? = readUInt8()?.toByte()

    fun readInt16(): Short? =
        if (!has(2)) null else buffer.short

    fun readInt32(): Int? =
        if (!has(4)) null else buffer.int

    fun readInt64(): Long? =
        if (!has(8)) null else buffer.long

    fun readUInt8(): UByte? =
        if (!has(1)) null else buffer.get().toUByte()

    fun readUInt16(): UShort? = readInt16()?.toUShort()

    fun readUInt32(): UInt? = readInt32()?.toUInt()

    fun readUInt64(): ULong? = readInt64()?.toULong()

    fun readFloat32(): Float? = readInt32()?.let { java.lang.Float.intBitsToFloat(it) }

    fun readFloat64(): Double? = readInt64()?.let { java.lang.Double.longBitsToDouble(it) }
}

/** Bytewise comparison used to order encoded collections canonically. */
public fun nexaCompareBytes(left: ByteArray, right: ByteArray): Int {
    val shared = minOf(left.size, right.size)
    for (index in 0 until shared) {
        val difference = (left[index].toInt() and 0xFF) - (right[index].toInt() and 0xFF)
        if (difference != 0) {
            return difference
        }
    }
    return left.size - right.size
}

"#;

#[cfg(test)]
mod tests {
    use nexa_ir::{Action, Expr, NumericType, PluginCodec, Type};

    use super::{Direction, codec_name, collect};

    /// A module whose only generic call sits in a lifecycle action rather than
    /// a body, which is where a call to a plugin that restores state belongs.
    fn module_with_lifecycle_call() -> nexa_ir::Module {
        nexa_ir::Module {
            app_name: "Demo".to_owned(),
            plugins: Vec::new(),
            plugin_assets: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            states: Vec::new(),
            screens: Vec::new(),
            components: Vec::new(),
            body: Vec::new(),
            status_bar: None,
            direction: None,
            on_appear: Some(vec![Action::Expression(Expr::NativeCall {
                receiver: None,
                namespace: "Store".to_owned(),
                name: "getObject".to_owned(),
                arguments: Vec::new(),
                codecs: vec![PluginCodec {
                    ty: Type::Numeric(NumericType::Float64),
                    decodes: true,
                }],
                return_type: Type::Optional(Box::new(Type::Numeric(NumericType::Float64))),
                is_async: false,
                is_throwing: false,
            })]),
            on_appear_async: false,
            on_disappear: None,
            on_active: None,
            on_inactive: None,
            on_background: None,
        }
    }

    /// A codec is emitted because a call uses it, so a call the walk does not
    /// reach would leave the generated call site pointing at a function that was
    /// never written.
    #[test]
    fn a_lifecycle_call_collects_its_codec() {
        let codecs = collect(&module_with_lifecycle_call());
        assert_eq!(
            codecs
                .iter()
                .map(|codec| codec_name(&codec.ty, codec.direction))
                .collect::<Vec<_>>(),
            ["nexaReadfloat64"]
        );
    }

    /// Nested types are collected before the type that calls them, so a codec
    /// is always declared after the codecs it uses.
    #[test]
    fn nested_codecs_come_before_the_collection_that_uses_them() {
        let mut module = module_with_lifecycle_call();
        let call = match &mut module.on_appear.as_mut().expect("lifecycle actions")[0] {
            Action::Expression(expression) => expression,
            _ => unreachable!("the fixture is an expression"),
        };
        if let Expr::NativeCall { codecs, .. } = call {
            codecs.clear();
            codecs.push(PluginCodec {
                ty: Type::Map(
                    Box::new(Type::String),
                    Box::new(Type::Array(Box::new(Type::Bool))),
                ),
                decodes: false,
            });
        }
        let names = collect(&module)
            .iter()
            .map(|codec| codec_name(&codec.ty, codec.direction))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "nexaWritestring",
                "nexaWritebool",
                "nexaWritearray_bool",
                "nexaWritemap_string_array_bool",
            ],
            "a collection's element codecs must be declared before the collection's own"
        );
    }

    /// A type no call passes to a generic method costs nothing: only the
    /// direction a call actually uses is collected.
    #[test]
    fn only_the_direction_a_call_uses_is_collected() {
        let module = module_with_lifecycle_call();
        let codecs = collect(&module);
        assert_eq!(codecs.len(), 1);
        assert_eq!(codecs[0].direction, Direction::Read);
    }

    /// A module with no generic call emits no codec at all.
    #[test]
    fn a_module_without_generic_calls_emits_nothing() {
        let mut module = module_with_lifecycle_call();
        module.on_appear = None;
        assert!(collect(&module).is_empty());
    }
}
