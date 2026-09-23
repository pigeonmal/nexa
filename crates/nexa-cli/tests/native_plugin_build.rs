use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempProject(PathBuf);

impl TempProject {
    fn new(platform: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        Self(env::temp_dir().join(format!(
            "nexa-plugin-build-{platform}-{}-{nonce}",
            std::process::id()
        )))
    }

    fn generate(&self, target: &str) -> PathBuf {
        let output = self.0.join("Generated");
        let entry = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/plugins/video-player-demo.nx");
        let result = Command::new(env!("CARGO_BIN_EXE_nexa"))
            .arg("generate")
            .arg(entry)
            .arg("--target")
            .arg(target)
            .arg("--out")
            .arg(&output)
            .arg("--name")
            .arg("NexaPluginBuildTest")
            .output()
            .expect("Nexa CLI should start");
        assert!(
            result.status.success(),
            "Nexa project generation failed:\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        output
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command_available(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn gradle_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GRADLE").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    if command_available("gradle", &["--version"]) {
        return Some(PathBuf::from("gradle"));
    }

    #[cfg(windows)]
    let executable = "gradle.bat";
    #[cfg(not(windows))]
    let executable = "gradle";

    let gradle_user_home = env::var_os("GRADLE_USER_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))?;
    let distributions = fs::read_dir(gradle_user_home.join("wrapper/dists")).ok()?;
    let mut candidates = Vec::new();

    for distribution in distributions.filter_map(Result::ok) {
        if !distribution.path().is_dir() {
            continue;
        }
        let hashes = fs::read_dir(distribution.path()).ok()?;
        for hash in hashes.filter_map(Result::ok) {
            if !hash.path().is_dir() {
                continue;
            }
            let versions = fs::read_dir(hash.path()).ok()?;
            for version_dir in versions.filter_map(Result::ok) {
                if !version_dir.path().is_dir() {
                    continue;
                }
                let name = version_dir.file_name();
                let Some(version) = name.to_str().and_then(|name| name.strip_prefix("gradle-"))
                else {
                    continue;
                };
                let Some(version_key) = parse_version(version) else {
                    continue;
                };
                let path = version_dir.path().join("bin").join(executable);
                if path.is_file() {
                    candidates.push((version_key, path));
                }
            }
        }
    }

    candidates.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

fn parse_version(version: &str) -> Option<Vec<u64>> {
    version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|character| character.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

fn kotlin_compiler() -> Option<PathBuf> {
    if let Some(path) = env::var_os("KOTLINC").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    if command_available("kotlinc", &["-version"]) {
        return Some(PathBuf::from("kotlinc"));
    }
    let android_studio_kotlinc = PathBuf::from(
        "/Applications/Android Studio.app/Contents/plugins/Kotlin/kotlinc/bin/kotlinc",
    );
    android_studio_kotlinc
        .is_file()
        .then_some(android_studio_kotlinc)
}

fn android_ndk_compiler(sdk: &Path) -> Option<PathBuf> {
    let mut ndks = fs::read_dir(sdk.join("ndk"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    ndks.sort();
    ndks.into_iter().rev().find_map(|ndk| {
        let compiler =
            ndk.join("toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android26-clang++");
        compiler.is_file().then_some(compiler)
    })
}

fn source_tree_contains(root: &Path, extension: &str, needle: &str) -> bool {
    fs::read_dir(root)
        .expect("generated source directory should be readable")
        .filter_map(Result::ok)
        .any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                source_tree_contains(&path, extension, needle)
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                fs::read_to_string(path).is_ok_and(|source| source.contains(needle))
            } else {
                false
            }
        })
}

fn source_text_containing(root: &Path, extension: &str, needle: &str) -> Option<String> {
    for entry in fs::read_dir(root).ok()?.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(source) = source_text_containing(&path, extension, needle) {
                return Some(source);
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            if let Ok(source) = fs::read_to_string(path) {
                if source.contains(needle) {
                    return Some(source);
                }
            }
        }
    }
    None
}

#[derive(Clone, Copy)]
struct PrimitiveTypeCase {
    idl: &'static str,
    suffix: &'static str,
    cpp: &'static str,
    kotlin: &'static str,
    scalar_value: &'static str,
    array_value: &'static str,
    swift_value: &'static str,
}

const CXX_PRIMITIVE_TYPES: &[PrimitiveTypeCase] = &[
    PrimitiveTypeCase {
        idl: "Bool",
        suffix: "Bool",
        cpp: "bool",
        kotlin: "Boolean",
        scalar_value: "true",
        array_value: "listOf(false, true)",
        swift_value: "true",
    },
    PrimitiveTypeCase {
        idl: "Int8",
        suffix: "Int8",
        cpp: "std::int8_t",
        kotlin: "Byte",
        scalar_value: "Byte.MIN_VALUE",
        array_value: "listOf(Byte.MIN_VALUE, 0, Byte.MAX_VALUE)",
        swift_value: "Int8.min",
    },
    PrimitiveTypeCase {
        idl: "Int16",
        suffix: "Int16",
        cpp: "std::int16_t",
        kotlin: "Short",
        scalar_value: "Short.MIN_VALUE",
        array_value: "listOf(Short.MIN_VALUE, 0, Short.MAX_VALUE)",
        swift_value: "Int16.min",
    },
    PrimitiveTypeCase {
        idl: "Int32",
        suffix: "Int32",
        cpp: "std::int32_t",
        kotlin: "Int",
        scalar_value: "Int.MIN_VALUE",
        array_value: "listOf(Int.MIN_VALUE, 0, Int.MAX_VALUE)",
        swift_value: "Int32.min",
    },
    PrimitiveTypeCase {
        idl: "Int64",
        suffix: "Int64",
        cpp: "std::int64_t",
        kotlin: "Long",
        scalar_value: "Long.MIN_VALUE",
        array_value: "listOf(Long.MIN_VALUE, 0L, Long.MAX_VALUE)",
        swift_value: "Int64.min",
    },
    PrimitiveTypeCase {
        idl: "UInt8",
        suffix: "UInt8",
        cpp: "std::uint8_t",
        kotlin: "UByte",
        scalar_value: "UByte.MAX_VALUE",
        array_value: "listOf(0u, UByte.MAX_VALUE)",
        swift_value: "UInt8.max",
    },
    PrimitiveTypeCase {
        idl: "UInt16",
        suffix: "UInt16",
        cpp: "std::uint16_t",
        kotlin: "UShort",
        scalar_value: "UShort.MAX_VALUE",
        array_value: "listOf(0u, UShort.MAX_VALUE)",
        swift_value: "UInt16.max",
    },
    PrimitiveTypeCase {
        idl: "UInt32",
        suffix: "UInt32",
        cpp: "std::uint32_t",
        kotlin: "UInt",
        scalar_value: "UInt.MAX_VALUE",
        array_value: "listOf(0u, UInt.MAX_VALUE)",
        swift_value: "UInt32.max",
    },
    PrimitiveTypeCase {
        idl: "UInt64",
        suffix: "UInt64",
        cpp: "std::uint64_t",
        kotlin: "ULong",
        scalar_value: "ULong.MAX_VALUE",
        array_value: "listOf(0uL, ULong.MAX_VALUE)",
        swift_value: "UInt64.max",
    },
    PrimitiveTypeCase {
        idl: "Float32",
        suffix: "Float32",
        cpp: "float",
        kotlin: "Float",
        scalar_value: "-1.25f",
        array_value: "listOf(-1.25f, 0f, Float.MAX_VALUE)",
        swift_value: "Float.greatestFiniteMagnitude",
    },
    PrimitiveTypeCase {
        idl: "Float64",
        suffix: "Float64",
        cpp: "double",
        kotlin: "Double",
        scalar_value: "-1.25",
        array_value: "listOf(-1.25, 0.0, Double.MAX_VALUE)",
        swift_value: "Double.greatestFiniteMagnitude",
    },
    PrimitiveTypeCase {
        idl: "String",
        suffix: "String",
        cpp: "std::string",
        kotlin: "String",
        scalar_value: "text",
        array_value: "listOf(\"\", \"Nexa 🚀\")",
        swift_value: "\"Nexa 🚀\"",
    },
    PrimitiveTypeCase {
        idl: "Bytes",
        suffix: "Bytes",
        cpp: "std::vector<std::uint8_t>",
        kotlin: "ByteArray",
        scalar_value: "bytes",
        array_value: "listOf(byteArrayOf(0, -1), byteArrayOf())",
        swift_value: "Data([0, 255])",
    },
];

fn type_matrix_idl(include_android_nested_map: bool) -> String {
    let mut idl = String::from("service Types {\n    fn ping()\n");
    for ty in CXX_PRIMITIVE_TYPES {
        idl.push_str(&format!(
            "    fn echo{0}(value: {1}) -> {1}\n    fn maybe{0}(value: {1}?) -> {1}?\n    fn echoArray{0}(values: Array<{1}>) -> Array<{1}>\n    fn echoNestedArray{0}(values: Array<Array<{1}>>) -> Array<Array<{1}>>\n",
            ty.suffix, ty.idl
        ));
        if !matches!(ty.idl, "Float32" | "Float64" | "String" | "Bytes") {
            idl.push_str(&format!(
                "    fn echoMap{0}(values: Map<{1}, {1}>) -> Map<{1}, {1}>\n",
                ty.suffix, ty.idl
            ));
            idl.push_str(&format!(
                "    fn echoSet{0}(values: Set<{1}>) -> Set<{1}>\n",
                ty.suffix, ty.idl
            ));
        }
    }
    idl.push_str(
        "    fn echoMapFloat32(values: Map<Bool, Float32>) -> Map<Bool, Float32>\n    fn echoMapFloat64(values: Map<Int8, Float64>) -> Map<Int8, Float64>\n    fn echoMapString(values: Map<Int16, String>) -> Map<Int16, String>\n    fn echoBytesMap(values: Map<Int32, Bytes>) -> Map<Int32, Bytes>\n    fn echoArrayMap(values: Map<Int32, Array<Int32>>) -> Map<Int32, Array<Int32>>\n    fn echoStringArrayMap(values: Map<Int32, Array<String>>) -> Map<Int32, Array<String>>\n    fn echoBytesArrayMap(values: Map<Int32, Array<Bytes>>) -> Map<Int32, Array<Bytes>>\n    fn echoSetMap(values: Map<Int32, Set<UInt32>>) -> Map<Int32, Set<UInt32>>\n    fn echoArraySet(values: Array<Set<UInt32>>) -> Array<Set<UInt32>>\n",
    );
    idl.push_str("    fn echoNestedArrayMap(values: Map<Int32, Array<Array<Int32>>>) -> Map<Int32, Array<Array<Int32>>>\n");
    if include_android_nested_map {
        for ty in CXX_PRIMITIVE_TYPES {
            idl.push_str(&format!(
                "    fn echoOptionalArray{0}(values: Array<{1}?>) -> Array<{1}?>\n",
                ty.suffix, ty.idl
            ));
        }
        idl.push_str(
            "    fn echoOptionalSet(values: Set<UInt32?>) -> Set<UInt32?>\n    fn echoNullableValueMap(values: Map<Int32, Int32?>) -> Map<Int32, Int32?>\n    fn echoNullableArrayMap(values: Map<Int32, Array<Int32?>>) -> Map<Int32, Array<Int32?>>\n    fn echoNullableSetMap(values: Map<Int32, Set<UInt32?>>) -> Map<Int32, Set<UInt32?>>\n    fn echoNestedMap(values: Map<Int32, Map<String, Bytes>>) -> Map<Int32, Map<String, Bytes>>\n    fn echoOptionalMap(values: Map<UInt32, Bytes>?) -> Map<UInt32, Bytes>?\n    fn echoArrayMapValues(values: Map<Int32, Array<Map<String, Array<Int32>>>>) -> Map<Int32, Array<Map<String, Array<Int32>>>>\n    fn echoArraySetValues(values: Map<Int32, Array<Set<UInt32>>>) -> Map<Int32, Array<Set<UInt32>>>\n    fn echoMapArray(values: Array<Map<String, Array<Int32>>>) -> Array<Map<String, Array<Int32>>>\n",
        );
        idl.push_str("    fn echoOptionalNestedMap(values: Map<Int32, Map<String, Bytes>?>) -> Map<Int32, Map<String, Bytes>?>\n");
    }
    idl.push_str("}\n");
    idl
}

fn type_matrix_cpp_source(include_android_nested_map: bool) -> String {
    let mut source = String::from(
        "#include \"NexaPluginBindings.hpp\"\n#include <cstdint>\n#include <optional>\n#include <string>\n#include <utility>\n#include <vector>\nnamespace plugin_dev::plugin_example::plugin_cpp_dash_type_dash_matrix {\nnamespace Types {\nvoid ping() noexcept {}\n",
    );
    for ty in CXX_PRIMITIVE_TYPES {
        source.push_str(&format!(
            "{0} echo{1}({0} value) noexcept {{ return value; }}\nstd::optional<{0}> maybe{1}(std::optional<{0}> value) noexcept {{ return value; }}\nstd::vector<{0}> echoArray{1}(std::vector<{0}> values) noexcept {{ return values; }}\nstd::vector<std::vector<{0}>> echoNestedArray{1}(std::vector<std::vector<{0}>> values) noexcept {{ return values; }}\n",
            ty.cpp, ty.suffix
        ));
        if include_android_nested_map {
            source.push_str(&format!(
                "std::vector<std::optional<{0}>> echoOptionalArray{1}(std::vector<std::optional<{0}>> values) noexcept {{ return values; }}\n",
                ty.cpp, ty.suffix
            ));
        }
        if !matches!(ty.idl, "Float32" | "Float64" | "String" | "Bytes") {
            source.push_str(&format!(
                "std::map<{0}, {0}> echoMap{1}(std::map<{0}, {0}> values) noexcept {{ return values; }}\nstd::set<{0}> echoSet{1}(std::set<{0}> values) noexcept {{ return values; }}\n",
                ty.cpp, ty.suffix
            ));
        }
    }
    source.push_str(
        "std::map<bool, float> echoMapFloat32(std::map<bool, float> values) noexcept { return values; }\nstd::map<std::int8_t, double> echoMapFloat64(std::map<std::int8_t, double> values) noexcept { return values; }\nstd::map<std::int16_t, std::string> echoMapString(std::map<std::int16_t, std::string> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::uint8_t>> echoBytesMap(std::map<std::int32_t, std::vector<std::uint8_t>> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::int32_t>> echoArrayMap(std::map<std::int32_t, std::vector<std::int32_t>> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::string>> echoStringArrayMap(std::map<std::int32_t, std::vector<std::string>> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::vector<std::uint8_t>>> echoBytesArrayMap(std::map<std::int32_t, std::vector<std::vector<std::uint8_t>>> values) noexcept { return values; }\nstd::map<std::int32_t, std::set<std::uint32_t>> echoSetMap(std::map<std::int32_t, std::set<std::uint32_t>> values) noexcept { return values; }\nstd::vector<std::set<std::uint32_t>> echoArraySet(std::vector<std::set<std::uint32_t>> values) noexcept { return values; }\n",
    );
    source.push_str("std::map<std::int32_t, std::vector<std::vector<std::int32_t>>> echoNestedArrayMap(std::map<std::int32_t, std::vector<std::vector<std::int32_t>>> values) noexcept { return values; }\n");
    if include_android_nested_map {
        source.push_str("std::set<std::optional<std::uint32_t>> echoOptionalSet(std::set<std::optional<std::uint32_t>> values) noexcept { return values; }\nstd::map<std::int32_t, std::optional<std::int32_t>> echoNullableValueMap(std::map<std::int32_t, std::optional<std::int32_t>> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::optional<std::int32_t>>> echoNullableArrayMap(std::map<std::int32_t, std::vector<std::optional<std::int32_t>>> values) noexcept { return values; }\nstd::map<std::int32_t, std::set<std::optional<std::uint32_t>>> echoNullableSetMap(std::map<std::int32_t, std::set<std::optional<std::uint32_t>>> values) noexcept { return values; }\n");
        source.push_str("std::map<std::int32_t, std::vector<std::map<std::string, std::vector<std::int32_t>>>> echoArrayMapValues(std::map<std::int32_t, std::vector<std::map<std::string, std::vector<std::int32_t>>>> values) noexcept { return values; }\nstd::map<std::int32_t, std::vector<std::set<std::uint32_t>>> echoArraySetValues(std::map<std::int32_t, std::vector<std::set<std::uint32_t>>> values) noexcept { return values; }\nstd::vector<std::map<std::string, std::vector<std::int32_t>>> echoMapArray(std::vector<std::map<std::string, std::vector<std::int32_t>>> values) noexcept { return values; }\n");
        source.push_str("std::map<std::int32_t, std::map<std::string, std::vector<std::uint8_t>>> echoNestedMap(std::map<std::int32_t, std::map<std::string, std::vector<std::uint8_t>>> values) noexcept { return values; }\n");
        source.push_str("std::optional<std::map<std::uint32_t, std::vector<std::uint8_t>>> echoOptionalMap(std::optional<std::map<std::uint32_t, std::vector<std::uint8_t>>> values) noexcept { return values; }\n");
        source.push_str("std::map<std::int32_t, std::optional<std::map<std::string, std::vector<std::uint8_t>>>> echoOptionalNestedMap(std::map<std::int32_t, std::optional<std::map<std::string, std::vector<std::uint8_t>>>> values) noexcept { return values; }\n");
    }
    source.push_str("}\n}\n");
    source
}

fn type_matrix_plugin(temp: &TempProject, include_android_nested_map: bool) -> (PathBuf, PathBuf) {
    let plugin = temp.0.join("cpp-plugin");
    fs::create_dir_all(plugin.join("cpp/Sources"))
        .expect("C++ type-matrix source directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.cpp-type-matrix"
    version: "1.0.0"
    sources { native: "native.nxid" }
    cpp { standard: "c++20" sources: ["cpp/Sources/**"] }
}
"#,
    )
    .expect("C++ type-matrix manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        type_matrix_idl(include_android_nested_map),
    )
    .expect("C++ type-matrix IDL should be written");
    let cpp = plugin.join("cpp/Sources/Plugin.cpp");
    fs::write(&cpp, type_matrix_cpp_source(include_android_nested_map))
        .expect("C++ type-matrix implementation should be written");
    let entry = temp.0.join("main.nx");
    fs::write(
        &entry,
        "plugin \"cpp-plugin\" as Types\napp TypeMatrix { body { Button(\"Ping\") { Types.ping() } } }\n",
    )
    .expect("C++ type-matrix app should be written");
    (entry, cpp)
}

fn type_matrix_kotlin_smoke() -> String {
    let mut source = String::from(
        "package com.nexa.cpptypematrixandroid\n\nfun main() {\n    val text = \"Nexa 🚀\"\n    val bytes = byteArrayOf(0, -1, 127, -128)\n    TypesPlugin.ping()\n",
    );
    for ty in CXX_PRIMITIVE_TYPES {
        let value = if ty.idl == "String" {
            "text"
        } else if ty.idl == "Bytes" {
            "bytes"
        } else {
            ty.scalar_value
        };
        if ty.idl == "Bytes" {
            source.push_str(&format!(
                "    check(TypesPlugin.echoBytes({value}).contentEquals({value}))\n"
            ));
        } else {
            source.push_str(&format!(
                "    check(TypesPlugin.echo{0}({value}) == {value})\n",
                ty.suffix
            ));
        }
        let maybe_value = if ty.idl == "String" {
            "text"
        } else if ty.idl == "Bytes" {
            "bytes"
        } else {
            ty.scalar_value
        };
        if ty.idl == "Bytes" {
            source.push_str(&format!(
                "    check(TypesPlugin.maybeBytes({maybe_value})!!.contentEquals({maybe_value}))\n"
            ));
        } else {
            source.push_str(&format!(
                "    check(TypesPlugin.maybe{0}({maybe_value}) == {maybe_value})\n",
                ty.suffix
            ));
        }
        source.push_str(&format!(
            "    check(TypesPlugin.maybe{}(null) == null)\n",
            ty.suffix
        ));
        source.push_str(&format!(
            "    val array{0} = {1}\n",
            ty.suffix, ty.array_value
        ));
        let optional_element = if ty.idl == "String" {
            "text"
        } else if ty.idl == "Bytes" {
            "bytes"
        } else {
            ty.scalar_value
        };
        source.push_str(&format!(
            "    val optionalArray{0}: List<{1}?> = listOf(null, {2}, {2})\n    val returnedOptionalArray{0} = TypesPlugin.echoOptionalArray{0}(optionalArray{0})\n",
            ty.suffix, ty.kotlin, optional_element
        ));
        if ty.idl == "Bytes" {
            source.push_str(&format!(
                "    check(returnedOptionalArray{0}.size == optionalArray{0}.size && returnedOptionalArray{0}.indices.all {{ index -> val actual = returnedOptionalArray{0}[index]; val expected = optionalArray{0}[index]; if (actual == null || expected == null) actual == expected else actual.contentEquals(expected) }})\n",
                ty.suffix
            ));
        } else {
            source.push_str(&format!(
                "    check(returnedOptionalArray{0} == optionalArray{0})\n",
                ty.suffix
            ));
        }
        if ty.idl == "Bytes" {
            source.push_str(&format!(
                "    val returnedBytes = TypesPlugin.echoArrayBytes(arrayBytes)\n    check(returnedBytes.size == arrayBytes.size && returnedBytes.indices.all {{ returnedBytes[it].contentEquals(arrayBytes[it]) }})\n"
            ));
        } else {
            source.push_str(&format!(
                "    check(TypesPlugin.echoArray{0}(array{0}) == array{0})\n",
                ty.suffix
            ));
        }
        source.push_str(&format!(
            "    val nestedArray{0} = listOf(array{0}, emptyList(), listOf({1}))\n",
            ty.suffix, ty.scalar_value
        ));
        if ty.idl == "Bytes" {
            source.push_str(
                "    val returnedNestedArrayBytes = TypesPlugin.echoNestedArrayBytes(nestedArrayBytes)\n    check(returnedNestedArrayBytes.size == nestedArrayBytes.size && returnedNestedArrayBytes.indices.all { row -> returnedNestedArrayBytes[row].size == nestedArrayBytes[row].size && returnedNestedArrayBytes[row].indices.all { column -> returnedNestedArrayBytes[row][column].contentEquals(nestedArrayBytes[row][column]) } })\n",
            );
        } else {
            source.push_str(&format!(
                "    check(TypesPlugin.echoNestedArray{0}(nestedArray{0}) == nestedArray{0})\n",
                ty.suffix
            ));
        }
        if !matches!(ty.idl, "Float32" | "Float64" | "String" | "Bytes") {
            let value = if ty.idl == "Bool" {
                "true"
            } else {
                ty.scalar_value
            };
            source.push_str(&format!(
                "    val map{0} = mapOf({1} to {1})\n    check(TypesPlugin.echoMap{0}(map{0}) == map{0})\n",
                ty.suffix, value
            ));
            source.push_str(&format!(
                "    val set{0} = setOf({1})\n    check(TypesPlugin.echoSet{0}(set{0}) == set{0})\n",
                ty.suffix, value
            ));
        }
    }
    source.push_str(
        "    val mapFloat32 = mapOf(false to -1.25f, true to Float.MAX_VALUE)\n    check(TypesPlugin.echoMapFloat32(mapFloat32) == mapFloat32)\n    val mapFloat64 = mapOf(Byte.MIN_VALUE to -1.25, Byte.MAX_VALUE to Double.MAX_VALUE)\n    check(TypesPlugin.echoMapFloat64(mapFloat64) == mapFloat64)\n    val mapString = mapOf(Short.MIN_VALUE to text)\n    check(TypesPlugin.echoMapString(mapString) == mapString)\n    val byteMap = mapOf(1 to bytes, 2 to byteArrayOf())\n    val returnedByteMap = TypesPlugin.echoBytesMap(byteMap)\n    check(returnedByteMap.keys == byteMap.keys && returnedByteMap.all { (key, value) -> value.contentEquals(byteMap.getValue(key)) })\n    check(TypesPlugin.echoBytesMap(emptyMap()).isEmpty())\n    val arrayMap = mapOf(1 to listOf(Int.MIN_VALUE, 0, Int.MAX_VALUE), 2 to emptyList())\n    check(TypesPlugin.echoArrayMap(arrayMap) == arrayMap)\n    check(TypesPlugin.echoArrayMap(emptyMap()).isEmpty())\n    val stringArrayMap = mapOf(1 to listOf(\"\", text), 2 to emptyList())\n    check(TypesPlugin.echoStringArrayMap(stringArrayMap) == stringArrayMap)\n    val bytesArrayMap = mapOf(1 to listOf(bytes, byteArrayOf()), 2 to emptyList())\n    val returnedBytesArrayMap = TypesPlugin.echoBytesArrayMap(bytesArrayMap)\n    check(returnedBytesArrayMap.keys == bytesArrayMap.keys && returnedBytesArrayMap.all { (key, values) -> values.size == bytesArrayMap.getValue(key).size && values.indices.all { values[it].contentEquals(bytesArrayMap.getValue(key)[it]) } })\n    val setMap = mapOf(1 to setOf(0u, UInt.MAX_VALUE), Int.MIN_VALUE to emptySet())\n    check(TypesPlugin.echoSetMap(setMap) == setMap)\n    check(TypesPlugin.echoSetMap(emptyMap()).isEmpty())\n    val optionalByteMap: Map<UInt, ByteArray>? = mapOf(9u to bytes)\n    val returnedOptionalByteMap = TypesPlugin.echoOptionalMap(optionalByteMap)\n    check(returnedOptionalByteMap != null && returnedOptionalByteMap.getValue(9u).contentEquals(bytes))\n    check(TypesPlugin.echoOptionalMap(null) == null)\n    check(TypesPlugin.echoOptionalMap(emptyMap())?.isEmpty() == true)\n    val nestedByteMap = mapOf(7 to mapOf(\"payload\" to bytes, \"empty\" to byteArrayOf()), 8 to emptyMap())\n    val returnedNestedByteMap = TypesPlugin.echoNestedMap(nestedByteMap)\n    check(returnedNestedByteMap.keys == nestedByteMap.keys && returnedNestedByteMap.all { (outerKey, values) -> values.keys == nestedByteMap.getValue(outerKey).keys && values.all { (innerKey, value) -> value.contentEquals(nestedByteMap.getValue(outerKey).getValue(innerKey)) } })\n    check(TypesPlugin.echoNestedMap(emptyMap()).isEmpty())\n",
    );
    source.push_str(
        "    val optionalNestedMap: Map<Int, Map<String, ByteArray>?> = mapOf(1 to null, 2 to emptyMap(), 3 to mapOf(\"data\" to bytes))\n    val returnedOptionalNestedMap = TypesPlugin.echoOptionalNestedMap(optionalNestedMap)\n    check(returnedOptionalNestedMap.keys == optionalNestedMap.keys)\n    check(returnedOptionalNestedMap[1] == null)\n    check(returnedOptionalNestedMap[2]?.isEmpty() == true)\n    check(returnedOptionalNestedMap[3]?.get(\"data\")?.contentEquals(bytes) == true)\n",
    );
    source.push_str(
        "    val nestedArrayMap: Map<Int, List<List<Int>>> = mapOf(1 to listOf(listOf(Int.MIN_VALUE, Int.MAX_VALUE), emptyList()), 2 to emptyList())\n    check(TypesPlugin.echoNestedArrayMap(nestedArrayMap) == nestedArrayMap)\n    val arrayMapValues: Map<Int, List<Map<String, List<Int>>>> = mapOf(1 to listOf(mapOf(\"edge\" to listOf(Int.MIN_VALUE, Int.MAX_VALUE), \"empty\" to emptyList())), 2 to emptyList())\n    check(TypesPlugin.echoArrayMapValues(arrayMapValues) == arrayMapValues)\n    val arraySetValues: Map<Int, List<Set<UInt>>> = mapOf(1 to listOf(setOf(0u, UInt.MAX_VALUE), emptySet()), 2 to emptyList())\n    check(TypesPlugin.echoArraySetValues(arraySetValues) == arraySetValues)\n    val mapArray: List<Map<String, List<Int>>> = listOf(mapOf(\"numbers\" to listOf(Int.MIN_VALUE, Int.MAX_VALUE)), emptyMap())\n    check(TypesPlugin.echoMapArray(mapArray) == mapArray)\n",
    );
    source.push_str(
        "    val nullableValueMap: Map<Int, Int?> = mapOf(1 to null, 2 to Int.MIN_VALUE, 3 to Int.MAX_VALUE)\n    check(TypesPlugin.echoNullableValueMap(nullableValueMap) == nullableValueMap)\n    val nullableUIntSet: Set<UInt?> = setOf(null, 0u, UInt.MAX_VALUE)\n    check(TypesPlugin.echoOptionalSet(nullableUIntSet) == nullableUIntSet)\n    val nullableArrayMap: Map<Int, List<Int?>> = mapOf(1 to listOf(null, Int.MIN_VALUE, Int.MAX_VALUE), 2 to emptyList())\n    check(TypesPlugin.echoNullableArrayMap(nullableArrayMap) == nullableArrayMap)\n    val nullableSetMap: Map<Int, Set<UInt?>> = mapOf(1 to setOf(null, 0u, UInt.MAX_VALUE), 2 to emptySet())\n    check(TypesPlugin.echoNullableSetMap(nullableSetMap) == nullableSetMap)\n",
    );
    source.push_str("}\n");
    source
}

fn type_matrix_swift_probe() -> String {
    let mut source = String::from(
        "import Foundation\n\nfunc typeMatrixProbe() {\n    let api = TypesPlugin.shared\n    api.ping()\n    let text = \"Nexa 🚀\"\n    let bytes = Data([0, 255])\n",
    );
    for ty in CXX_PRIMITIVE_TYPES {
        let value = if ty.idl == "String" {
            "text"
        } else if ty.idl == "Bytes" {
            "bytes"
        } else {
            ty.swift_value
        };
        source.push_str(&format!(
            "    _ = api.echo{0}(value: {1})\n    _ = api.maybe{0}(value: {1})\n    _ = api.maybe{0}(value: nil)\n    _ = api.echoArray{0}(values: [{1}])\n    _ = api.echoNestedArray{0}(values: [[{1}], [], [{1}]])\n",
            ty.suffix, value
        ));
        if !matches!(ty.idl, "Float32" | "Float64" | "String" | "Bytes") {
            source.push_str(&format!(
                "    _ = api.echoMap{0}(values: [{1}: {1}])\n",
                ty.suffix, value
            ));
            source.push_str(&format!(
                "    _ = api.echoSet{0}(values: Set([{1}]))\n",
                ty.suffix, value
            ));
        }
    }
    source.push_str(
        "    _ = api.echoMapFloat32(values: [false: 1.25])\n    _ = api.echoMapFloat64(values: [-1: 2.5])\n    _ = api.echoMapString(values: [-2: \"Nexa\"])\n    _ = api.echoBytesMap(values: [1: bytes])\n    _ = api.echoArrayMap(values: [1: [Int32.min, 0, Int32.max], 2: []])\n    _ = api.echoStringArrayMap(values: [1: [\"\", \"Nexa 🚀\"], 2: []])\n    _ = api.echoBytesArrayMap(values: [1: [Data([0, 255]), Data()], 2: []])\n    _ = api.echoSetMap(values: [1: Set([UInt32.min, UInt32.max])])\n    _ = api.echoArraySet(values: [Set([UInt32.min, UInt32.max]), []])\n",
    );
    source.push_str("    _ = api.echoNestedArrayMap(values: [1: [[Int32.min, Int32.max], []]])\n");
    source.push_str("}\n");
    source
}

#[test]
fn static_audit_reports_native_release_sizes_as_unmeasured_without_the_flag() {
    let temp = TempProject::new("audit-static");
    fs::create_dir_all(&temp.0).expect("temporary audit directory should be created");
    let entry = temp.0.join("main.nx");
    fs::write(&entry, "app AuditSample { body { Text(\"Audit\") } }\n")
        .expect("audit entry should be written");
    let report_path = temp.0.join("audit.json");
    let audit = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("audit")
        .arg(&entry)
        .args(["--target", "android", "--out"])
        .arg(&report_path)
        .output()
        .expect("Nexa CLI should start");
    assert!(
        audit.status.success(),
        "static audit should succeed without native build tools:\n{}\n{}",
        String::from_utf8_lossy(&audit.stdout),
        String::from_utf8_lossy(&audit.stderr)
    );
    let report = fs::read_to_string(report_path).expect("audit report should be written");
    assert!(report.contains("\"format\": 2"));
    assert!(report.contains("\"generatedSourceBytes\":"));
    assert!(report.contains("\"releaseMeasurements\": null"));
    assert!(!report.contains("\"nativeBinaryBytes\""));
}

#[test]
fn generated_typed_error_contracts_typecheck_with_swift_when_available() {
    if !command_available("swiftc", &["--version"]) {
        return;
    }

    let temp = TempProject::new("typed-errors");
    fs::create_dir_all(&temp.0).expect("temporary plugin directory should be created");
    let manifest = temp.0.join("plugin.config.nx");
    fs::write(
        &manifest,
        r#"plugin {
    schema: 2
    id: "dev.example.typed-errors"
    version: "1.0.0"
    sources { native: "native.nxid" }
}
"#,
    )
    .expect("temporary manifest should be written");
    let contract = temp.0.join("native.nxid");
    fs::write(
        &contract,
        r#"error PlayerError {
    invalidUrl
    decodingFailed(message: String)
            }
            native class VideoPlayer {
                init()
                async fn prepare(url: String, playbackRate: Float64) throws PlayerError
            }
            "#,
    )
    .expect("temporary IDL should be written");

    let bindings = temp.0.join("NexaPluginBindings.swift");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("plugin")
        .arg("generate")
        .arg(&contract)
        .args(["--target", "swift", "--out"])
        .arg(&bindings)
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "Swift contract generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_source =
        fs::read_to_string(&bindings).expect("generated Swift contract should be readable");
    assert!(
        generated_source.contains("playbackRate: Double"),
        "Swift Float64 declarations must use the canonical Swift Double spelling"
    );

    let implementation = temp.0.join("VideoPlayerImpl.swift");
    fs::write(
        &implementation,
        "@MainActor public final class VideoPlayerImpl: VideoPlayerSpec {\n    required public init() {}\n    public func prepare(url: String, playbackRate: Double) async throws(PlayerError) {}\n}\n",
    )
    .expect("Swift implementation should be written");

    let checked = Command::new("swiftc")
        .arg("-typecheck")
        .arg("-swift-version")
        .arg("6")
        .arg(&bindings)
        .arg(&implementation)
        .output()
        .expect("swiftc should start when available");
    assert!(
        checked.status.success(),
        "generated typed-error contract failed Swift type-checking:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    fs::write(
        &implementation,
        "@MainActor public final class VideoPlayerImpl: VideoPlayerSpec {\n    required public init() {}\n    public func prepare(url: String, playbackRate: Float) async throws(PlayerError) {}\n}\n",
    )
    .expect("invalid Swift implementation should be written");
    let rejected = Command::new("swiftc")
        .arg("-typecheck")
        .arg("-swift-version")
        .arg("6")
        .arg(&bindings)
        .arg(&implementation)
        .output()
        .expect("swiftc should start when available");
    assert!(
        !rejected.status.success(),
        "Swift must reject an implementation whose Float64 parameter is Float"
    );
}

#[test]
fn generated_kotlin_contracts_typecheck_with_kotlinc_when_available() {
    if !command_available("kotlinc", &["-version"]) {
        return;
    }

    let temp = TempProject::new("kotlin-contract");
    fs::create_dir_all(&temp.0).expect("temporary plugin directory should be created");
    let manifest = temp.0.join("plugin.config.nx");
    fs::write(
        &manifest,
        r#"plugin {
    schema: 2
    id: "dev.example.kotlin-contract"
    version: "1.0.0"
    sources { native: "native.nxid" }
}
"#,
    )
    .expect("temporary manifest should be written");
    let contract = temp.0.join("native.nxid");
    fs::write(
        &contract,
        r#"error PlayerError {
    invalidUrl
    decodingFailed(message: String)
    maybe(message: String?)
}
native class VideoPlayer {
    init()
    async fn prepare(url: String, playbackRate: Float64) throws PlayerError
}
"#,
    )
    .expect("native contract should be written");

    let bindings = temp.0.join("NexaPluginBindings.kt");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("plugin")
        .arg("generate")
        .arg(&contract)
        .args([
            "--target",
            "kotlin",
            "--package",
            "dev.example.video",
            "--out",
        ])
        .arg(&bindings)
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "Kotlin contract generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_source =
        fs::read_to_string(&bindings).expect("generated Kotlin contract should be readable");
    assert!(
        generated_source
            .contains("data class decodingFailed(override val message: String) : PlayerError()"),
        "Kotlin error message payloads must override Throwable.message"
    );
    assert!(
        generated_source
            .contains("data class maybe(override val message: String?) : PlayerError()")
    );

    let implementation = temp.0.join("VideoPlayerImpl.kt");
    fs::write(
        &implementation,
        "package dev.example.video\n\nclass VideoPlayerImpl : VideoPlayerSpec {\n    override suspend fun prepare(url: String, playbackRate: Double) {}\n}\n",
    )
    .expect("Kotlin implementation should be written");

    let checked = Command::new("kotlinc")
        .arg(&bindings)
        .arg(&implementation)
        .arg("-d")
        .arg(temp.0.join("plugin.jar"))
        .output()
        .expect("kotlinc should start when available");
    assert!(
        checked.status.success(),
        "generated Kotlin contract failed Kotlin compilation:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    fs::write(
        &implementation,
        "package dev.example.video\n\nclass VideoPlayerImpl : VideoPlayerSpec {\n    override suspend fun prepare(url: String, playbackRate: Float) {}\n}\n",
    )
    .expect("invalid Kotlin implementation should be written");
    let rejected = Command::new("kotlinc")
        .arg(&bindings)
        .arg(&implementation)
        .arg("-d")
        .arg(temp.0.join("invalid-plugin.jar"))
        .output()
        .expect("kotlinc should start when available");
    assert!(
        !rejected.status.success(),
        "Kotlin must reject an implementation whose Float64 parameter is Float"
    );
}

#[test]
fn generated_cpp_contracts_compile_with_a_typed_native_implementation_when_clang_is_available() {
    if !command_available("clang++", &["--version"]) {
        return;
    }

    let temp = TempProject::new("cpp-contract");
    fs::create_dir_all(&temp.0).expect("temporary plugin directory should be created");
    fs::write(
        temp.0.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.cpp-contract"
    version: "1.0.0"
    sources { native: "native.nxid" }
}
"#,
    )
    .expect("temporary manifest should be written");
    let contract = temp.0.join("native.nxid");
    fs::write(
        &contract,
        r#"struct PlayerOptions { autoplay: Bool }
enum PlayerState { idle, playing }
error PlayerError { invalidUrl, decodingFailed(message: String) }
native class VideoPlayer {
    init(options: PlayerOptions)
    readonly property state: PlayerState
    property volume: Float64
    event ended()
    async fn prepare(url: String) throws PlayerError
}
"#,
    )
    .expect("native contract should be written");

    let header = temp.0.join("NexaPluginBindings.hpp");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("plugin")
        .arg("generate")
        .arg(&contract)
        .args(["--target", "cpp", "--out"])
        .arg(&header)
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "C++ contract generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_header =
        fs::read_to_string(&header).expect("generated C++ contract should be readable");
    assert!(generated_header.contains("virtual double getVolume() const noexcept = 0;"));
    assert!(generated_header.contains("std::future<NexaResult<void, PlayerError>> prepare"));

    let implementation = temp.0.join("VideoPlayerImpl.cpp");
    fs::write(
        &implementation,
        r#"#include "NexaPluginBindings.hpp"

namespace plugin_dev::plugin_example::plugin_cpp_dash_contract {

class VideoPlayerImpl final : public VideoPlayerSpec {
public:
    explicit VideoPlayerImpl(PlayerOptions options) : volume_(options.autoplay ? 1.0 : 0.5) {}
    PlayerState getState() const noexcept override { return PlayerState::idle; }
    double getVolume() const noexcept override { return volume_; }
    void setVolume(double value) noexcept override { volume_ = value; }
    std::future<NexaResult<void, PlayerError>> prepare(std::string url) noexcept override {
        return std::async(std::launch::deferred, [url = std::move(url)] {
            if (url.empty()) {
                return NexaResult<void, PlayerError>::failure(
                    PlayerError{PlayerError::Value{PlayerError::InvalidUrlCase{}}});
            }
            return NexaResult<void, PlayerError>::success();
        });
    }
    void setOnEnded(std::function<void()> handler) noexcept override { onEnded_ = std::move(handler); }
private:
    double volume_;
    std::function<void()> onEnded_;
};

std::unique_ptr<VideoPlayerSpec> makeVideoPlayerImpl(PlayerOptions options) {
    return std::make_unique<VideoPlayerImpl>(options);
}

} // namespace plugin_dev::plugin_example::plugin_cpp_dash_contract
"#,
    )
    .expect("C++ implementation should be written");
    let checked = Command::new("clang++")
        .args(["-std=c++20", "-fsyntax-only", "-I"])
        .arg(&temp.0)
        .arg(&implementation)
        .output()
        .expect("clang++ should start when available");
    assert!(
        checked.status.success(),
        "generated C++ contract failed C++ type-checking:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn generated_video_plugin_builds_for_ios_when_xcode_is_available() {
    if !command_available("xcrun", &["--sdk", "iphonesimulator", "--show-sdk-path"]) {
        return;
    }

    let temp = TempProject::new("ios");
    let project = temp.generate("ios");
    let result = Command::new("xcodebuild")
        .args([
            "-quiet",
            "-project",
            "ios/NexaPluginBuildTest.xcodeproj",
            "-scheme",
            "NexaPluginBuildTest",
            "-configuration",
            "Release",
            "-sdk",
            "iphonesimulator",
            "-destination",
            "generic/platform=iOS Simulator",
            "-derivedDataPath",
        ])
        .arg(temp.0.join("DerivedData"))
        .arg("CODE_SIGNING_ALLOWED=NO")
        .arg("build")
        .current_dir(project)
        .output()
        .expect("xcodebuild should start when its simulator SDK is available");
    assert!(
        result.status.success(),
        "generated iOS plugin host failed to build:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn generated_network_certificate_pinning_compiles_for_ios_when_xcode_is_available() {
    if !command_available("xcrun", &["--sdk", "iphonesimulator", "--show-sdk-path"]) {
        return;
    }

    let temp = TempProject::new("ios-network-pinning");
    let output = temp.0.join("Generated");
    fs::create_dir_all(&temp.0).expect("network pinning test directory should be created");
    let entry = temp.0.join("network.nx");
    fs::write(
        &entry,
        r#"app NetworkPinTest {
    body {
        OnAppear async {
            try {
                await Network.fetch(url: "https://example.com")
            } catch {
            }
        }
        Text("Network")
    }
}

"#,
    )
    .expect("network pinning test app should be written");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(entry)
        .args(["--target", "ios", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("NexaNetworkPinTest")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "network-pinning test project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_swift = source_text_containing(
        &output.join("ios"),
        "swift",
        "NexaCertificatePin.sha256SPKI",
    )
    .expect("generated iOS source should contain certificate pinning code");

    if command_available("swiftc", &["--version"]) {
        let helper_start = generated_swift
            .find("enum NexaCertificatePin {")
            .expect("generated certificate pin helper should be present");
        let helper_end = generated_swift[helper_start..]
            .find("final class NexaURLSessionDelegate")
            .map(|offset| helper_start + offset)
            .expect("generated pin helper should end before its URLSession delegate");
        let helper = &generated_swift[helper_start..helper_end];
        let smoke_test = temp.0.join("CertificatePinSmoke.swift");
        let smoke_source = format!(
            r#"import CryptoKit
import Foundation
import Security

{helper}

@main
struct CertificatePinSmoke {{
    static func main() {{
        func der(_ tag: UInt8, _ content: [UInt8]) -> [UInt8] {{
            if content.count < 128 {{ return [tag, UInt8(content.count)] + content }}
            var length = content.count
            var encodedLength: [UInt8] = []
            while length > 0 {{
                encodedLength.insert(UInt8(length & 0xff), at: 0)
                length >>= 8
            }}
            return [tag, 0x80 | UInt8(encodedLength.count)] + encodedLength + content
        }}

        let spki = der(0x30, der(0x30, []) + der(0x03, [0] + Array(repeating: 0x42, count: 128)))
        let tbs = der(0x30, der(0xa0, der(0x02, [2])) + der(0x02, [1]) + der(0x30, []) + der(0x30, []) + der(0x30, []) + der(0x30, []) + spki)
        let certificate = Data(der(0x30, tbs + der(0x30, []) + der(0x03, [0])))
        let expected = SHA256.hash(data: Data(spki)).map {{ String(format: "%02x", $0) }}.joined()
        guard NexaCertificatePin.sha256SPKI(in: certificate) == expected else {{
            fatalError("SPKI extraction did not hash the encoded public-key structure")
        }}
        guard NexaCertificatePin.sha256SPKI(in: Data([0x30, 0x80])) == nil else {{
            fatalError("malformed DER must not produce a certificate pin")
        }}
    }}
}}
"#
        );
        fs::write(&smoke_test, smoke_source).expect("certificate pin smoke test should be written");
        let executable = temp.0.join("CertificatePinSmoke");
        let compiled = Command::new("swiftc")
            .args(["-parse-as-library", "-o"])
            .arg(&executable)
            .arg(&smoke_test)
            .arg("-framework")
            .arg("Security")
            .output()
            .expect("Swift compiler should start");
        assert!(
            compiled.status.success(),
            "generated SPKI parser failed to compile:\n{}\n{}",
            String::from_utf8_lossy(&compiled.stdout),
            String::from_utf8_lossy(&compiled.stderr)
        );
        let ran = Command::new(executable)
            .output()
            .expect("certificate pin smoke test should start");
        assert!(
            ran.status.success(),
            "generated SPKI parser failed its DER extraction smoke test:\n{}\n{}",
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        );
    }

    let result = Command::new("xcodebuild")
        .args([
            "-quiet",
            "-project",
            "ios/NexaNetworkPinTest.xcodeproj",
            "-scheme",
            "NexaNetworkPinTest",
            "-configuration",
            "Release",
            "-sdk",
            "iphonesimulator",
            "-destination",
            "generic/platform=iOS Simulator",
            "-derivedDataPath",
        ])
        .arg(temp.0.join("DerivedData"))
        .arg("CODE_SIGNING_ALLOWED=NO")
        .arg("build")
        .current_dir(&output)
        .output()
        .expect("xcodebuild should start when its simulator SDK is available");
    assert!(
        result.status.success(),
        "generated iOS networking and pinning helpers failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn generated_android_pin_decoder_validates_spki_hex_when_kotlinc_is_available() {
    let Some(kotlinc) = kotlin_compiler() else {
        return;
    };
    if !command_available("java", &["-version"]) {
        return;
    }

    let temp = TempProject::new("android-network-pinning");
    fs::create_dir_all(&temp.0).expect("Android pin test directory should be created");
    let entry = temp.0.join("network.nx");
    fs::write(
        &entry,
        r#"app NetworkPinTest {
    body {
        OnAppear async {
            try {
                await Network.fetch(url: "https://example.com")
            } catch {
            }
        }
        Text("Network")
    }
}
"#,
    )
    .expect("network pin test app should be written");
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "android", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("NexaAndroidPinTest")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "Android pin test project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated_kotlin = source_text_containing(
        &output.join("android"),
        "kt",
        "fun hexPin(value: String): ByteArray",
    )
    .expect("generated Kotlin should include its pin decoder");
    let decoder_start = generated_kotlin
        .find("fun hexPin(value: String): ByteArray")
        .expect("pin decoder declaration should be present");
    let decoder_end = generated_kotlin[decoder_start..]
        .find("public object NexaNetwork")
        .map(|offset| decoder_start + offset)
        .expect("pin decoder should end before NexaNetwork");
    let smoke_test = temp.0.join("PinDecoderSmoke.kt");
    let source = format!(
        r#"{}

fun main() {{
    check(hexPin("00".repeat(32)).contentEquals(ByteArray(32)))
    check(hexPin("aF".repeat(32)).contentEquals(ByteArray(32) {{ 0xaf.toByte() }}))
    check(runCatching {{ hexPin("0".repeat(63)) }}.exceptionOrNull() is IllegalArgumentException)
    check(runCatching {{ hexPin("gg".repeat(32)) }}.exceptionOrNull() is IllegalArgumentException)
}}
"#,
        &generated_kotlin[decoder_start..decoder_end]
    );
    fs::write(&smoke_test, source).expect("Kotlin pin smoke test should be written");
    let jar = temp.0.join("PinDecoderSmoke.jar");
    let compiled = Command::new(kotlinc)
        .arg(&smoke_test)
        .arg("-include-runtime")
        .arg("-d")
        .arg(&jar)
        .output()
        .expect("Kotlin compiler should start");
    assert!(
        compiled.status.success(),
        "generated Android pin decoder failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let ran = Command::new("java")
        .arg("-jar")
        .arg(&jar)
        .output()
        .expect("Kotlin pin smoke test should start");
    assert!(
        ran.status.success(),
        "generated Android pin decoder failed its hex validation smoke test:\n{}\n{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
}

#[test]
fn video_player_ios_instances_run_independently_in_a_headless_swift_smoke_test() {
    if !cfg!(target_os = "macos") || !command_available("swiftc", &["--version"]) {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/video-player");
    let implementation = root.join("ios/Sources/VideoPlayerImpl.swift");
    let smoke_test = root.join("tests/ios/VideoPlayerRuntimeSmoke.swift");
    let temp = TempProject::new("ios-video-runtime");
    fs::create_dir_all(&temp.0).expect("Swift smoke test output directory should be created");
    let executable = temp.0.join("VideoPlayerRuntimeSmoke");
    let compiled = Command::new("swiftc")
        .args(["-swift-version", "6", "-parse-as-library", "-o"])
        .arg(&executable)
        .arg(&implementation)
        .arg(&smoke_test)
        .args([
            "-framework",
            "AVFoundation",
            "-framework",
            "AVKit",
            "-framework",
            "SwiftUI",
        ])
        .output()
        .expect("Swift compiler should start");
    assert!(
        compiled.status.success(),
        "VideoPlayer Swift runtime smoke test failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );

    let ran = Command::new(executable)
        .output()
        .expect("headless VideoPlayer Swift smoke test should start");
    assert!(
        ran.status.success(),
        "two-instance VideoPlayer Swift smoke test failed:\n{}\n{}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
}

#[test]
fn generated_video_plugin_builds_for_android_when_gradle_and_sdk_are_available() {
    let Some(gradle) = gradle_executable() else {
        return;
    };
    let Some(android_sdk) = env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from)
    else {
        return;
    };
    let has_compile_platform = fs::read_dir(android_sdk.join("platforms"))
        .ok()
        .is_some_and(|platforms| {
            platforms.filter_map(Result::ok).any(|platform| {
                let name = platform.file_name();
                let name = name.to_string_lossy();
                (name == "android-37" || name.starts_with("android-37."))
                    && platform.path().join("android.jar").is_file()
            })
        });
    if !has_compile_platform {
        return;
    }

    let temp = TempProject::new("android");
    let project = temp.generate("android");
    let video_player_test = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../examples/plugins/video-player/android/src/test/kotlin/dev/nexa/videoplayer/VideoPlayerImplTest.kt",
    );
    let generated_test_dir = project.join("android/app/src/test/java/dev/nexa/videoplayer");
    fs::create_dir_all(&generated_test_dir)
        .expect("generated Android test package should be created");
    fs::copy(
        video_player_test,
        generated_test_dir.join("VideoPlayerImplTest.kt"),
    )
    .expect("headless VideoPlayer test should be staged");
    let app_gradle = project.join("android/app/build.gradle.kts");
    let app_gradle_source =
        fs::read_to_string(&app_gradle).expect("generated Android Gradle file should be readable");
    let app_gradle_source = app_gradle_source.replacen(
        "dependencies {\n",
        "dependencies {\n    testImplementation(\"junit:junit:4.13.2\")\n",
        1,
    );
    fs::write(app_gradle, app_gradle_source)
        .expect("JUnit dependency should be added to the generated test project");
    let result = Command::new(gradle)
        .args([
            "--no-daemon",
            ":app:testDebugUnitTest",
            ":app:assembleDebug",
        ])
        .current_dir(project.join("android"))
        .output()
        .expect("Gradle should start when Android tooling is available");
    assert!(
        result.status.success(),
        "generated Android plugin host failed to build:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn generated_projects_integrate_plugin_services_and_ios_entitlements() {
    let temp = TempProject::new("entitlements");
    let plugin = temp.0.join("secure-plugin");
    fs::create_dir_all(&plugin).expect("plugin directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.push"
    version: "1.0.0"
    sources { native: "native.nxid" }
    ios {
        sources: ["ios/Sources/**"]
        entitlements {
            "aps-environment": "development"
            "com.apple.developer.associated-domains": ["applinks:example.com"]
        }
        linkerFlags: ["-ObjC"]
    }
}

"#,
    )
    .expect("plugin manifest should be written");
    let swift_source = plugin.join("ios/Sources/PushPlugin.swift");
    fs::create_dir_all(swift_source.parent().expect("Swift source has a parent"))
        .expect("Swift source directory should be created");
    fs::write(
        swift_source,
        "public final class PushPlugin: Push, @unchecked Sendable {\n    public static let shared = PushPlugin()\n    private init() {}\n    public func register() {}\n}\n",
    )
    .expect("Swift implementation should be written");
    fs::write(
        plugin.join("native.nxid"),
        "service Push { fn register() }\n",
    )
    .expect("native IDL should be written");
    let kotlin_source = plugin.join("android/src/main/kotlin/dev/example/push/PushImpl.kt");
    fs::create_dir_all(kotlin_source.parent().expect("Kotlin source has a parent"))
        .expect("Kotlin source directory should be created");
    fs::write(
        kotlin_source,
        "package dev.example.push\nobject PushPlugin { val instance = this; fun register() {} }\n",
    )
    .expect("Kotlin implementation should be written");
    let entry = temp.0.join("main.nx");
    fs::write(
        &entry,
        "plugin \"secure-plugin\" as Push\napp Demo { body { Button(\"Register\") { Push.register() } } }\n",
    )
    .expect("Nexa app should be written");
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "all", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("EntitlementTest")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "iOS project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let entitlements = fs::read_to_string(output.join("ios/EntitlementTest/Nexa.entitlements"))
        .expect("generated entitlements file should exist");
    assert!(entitlements.contains("<key>aps-environment</key><string>development</string>"));
    assert!(entitlements.contains("applinks:example.com"));
    let project = fs::read_to_string(output.join("ios/EntitlementTest.xcodeproj/project.pbxproj"))
        .expect("generated Xcode project should exist");
    assert!(project.contains("CODE_SIGN_ENTITLEMENTS = EntitlementTest/Nexa.entitlements"));
    assert!(project.contains("OTHER_LDFLAGS = ( \"$(inherited)\", \"-ObjC\" );"));
    assert!(
        source_tree_contains(
            &output.join("android/app/src/main/java"),
            "kt",
            "PushPlugin.instance.register()"
        ),
        "Android output should keep the plugin call statically resolved"
    );
    if command_available("xcrun", &["--sdk", "iphonesimulator", "--show-sdk-path"])
        && command_available("xcodebuild", &["-version"])
    {
        let built = Command::new("xcodebuild")
            .args([
                "-quiet",
                "-project",
                "ios/EntitlementTest.xcodeproj",
                "-scheme",
                "EntitlementTest",
                "-sdk",
                "iphonesimulator",
                "-destination",
                "generic/platform=iOS Simulator",
                "-derivedDataPath",
            ])
            .arg(temp.0.join("EntitlementDerivedData"))
            .arg("CODE_SIGNING_ALLOWED=NO")
            .arg("build")
            .current_dir(&output)
            .output()
            .expect("xcodebuild should start when available");
        assert!(
            built.status.success(),
            "generated project with plugin linker flags and entitlements failed to build:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }

    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.push"
    version: "1.0.0"
    sources { native: "native.nxid" }
    ios { sources: ["ios/Sources/**"] linkerFlags: ["-ObjC"] }
}
"#,
    )
    .expect("updated plugin manifest should be written");
    let regenerated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "all", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("EntitlementTest")
        .output()
        .expect("Nexa CLI should regenerate the project");
    assert!(
        regenerated.status.success(),
        "iOS project regeneration failed:\n{}\n{}",
        String::from_utf8_lossy(&regenerated.stdout),
        String::from_utf8_lossy(&regenerated.stderr)
    );
    assert!(
        !output
            .join("ios/EntitlementTest/Nexa.entitlements")
            .exists(),
        "removing all plugin entitlements must remove the stale generated file"
    );
    let project = fs::read_to_string(output.join("ios/EntitlementTest.xcodeproj/project.pbxproj"))
        .expect("regenerated Xcode project should exist");
    assert!(
        !project.contains("CODE_SIGN_ENTITLEMENTS"),
        "regeneration without entitlements must clear the code-signing setting"
    );
}

#[test]
fn declared_cpp_sources_are_compiled_by_generated_ios_project_when_xcode_is_available() {
    let temp = TempProject::new("ios-cpp-source");
    let plugin = temp.0.join("cpp-plugin");
    let cpp_source = plugin.join("cpp/Sources/Probe.cpp");
    let cpp_header = plugin.join("cpp/include/details/Native.hpp");
    fs::create_dir_all(cpp_source.parent().expect("C++ source has a parent"))
        .expect("C++ source directory should be created");
    fs::create_dir_all(cpp_header.parent().expect("C++ header has a parent"))
        .expect("C++ header directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.cpp-source"
    version: "1.0.0"
    sources { native: "native.nxid" }
    cpp {
        standard: "c++23"
        sources: ["cpp/Sources/**"]
        headers: ["cpp/include/**"]
    }
}

"#,
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        r#"service Counter {
    fn ping()
    fn echo(value: Int32) -> Int32
    fn echoText(value: String) -> String
    fn echoBytes(value: Bytes) -> Bytes
    fn maybeNumber(value: Int32?) -> Int32?
    fn maybeText(value: String?) -> String?
    fn maybeBytes(value: Bytes?) -> Bytes?
    fn echoValues(values: Array<Int32>) -> Array<Int32>
    fn echoLabels(values: Array<String>) -> Array<String>
    fn echoPayloads(values: Array<Bytes>) -> Array<Bytes>
    fn echoIntSet(values: Set<Int32>) -> Set<Int32>
}
native class CounterValue {
    init(initial: Int32?, values: Set<Int32>)
    event changed(value: Int32, title: String, payload: Bytes)
    property value: Int32
    property title: String
    property payload: Bytes
    property maybeTitle: String?
    property maybePayload: Bytes?
    property samples: Array<Int32>
    property values: Set<Int32>
    fn echo(value: String) -> String
    fn echoBytes(value: Bytes) -> Bytes
    fn maybeEcho(value: String?) -> String?
    fn echoValues(values: Array<Int32>) -> Array<Int32>
    fn echoSet(values: Set<Int32>) -> Set<Int32>
    fn increment()
    fn notify(value: Int32)
}
"#,
    )
    .expect("native IDL should be written");
    fs::write(
        &cpp_header,
        "#pragma once\nnamespace details { inline int value() { return 7; } }\n",
    )
    .expect("C++ plugin header should be written");
    fs::write(
        &cpp_source,
        r#"#include "NexaPluginBindings.hpp"
#include "details/Native.hpp"
#include <string>
#include <thread>
#include <utility>
#include <vector>
namespace plugin_dev::plugin_example::plugin_cpp_dash_source {
namespace Counter {
void ping() noexcept {}
std::int32_t echo(std::int32_t value) noexcept { return value; }
std::string echoText(std::string value) noexcept { return value; }
std::vector<std::uint8_t> echoBytes(std::vector<std::uint8_t> value) noexcept { return value; }
std::optional<std::int32_t> maybeNumber(std::optional<std::int32_t> value) noexcept { return value; }
std::optional<std::string> maybeText(std::optional<std::string> value) noexcept { return value; }
std::optional<std::vector<std::uint8_t>> maybeBytes(std::optional<std::vector<std::uint8_t>> value) noexcept { return value; }
std::vector<std::int32_t> echoValues(std::vector<std::int32_t> values) noexcept { return values; }
std::vector<std::string> echoLabels(std::vector<std::string> values) noexcept { return values; }
std::vector<std::vector<std::uint8_t>> echoPayloads(std::vector<std::vector<std::uint8_t>> values) noexcept { return values; }
std::set<std::int32_t> echoIntSet(std::set<std::int32_t> values) noexcept { return values; }
}
class CounterValueImpl final : public CounterValueSpec {
public:
    CounterValueImpl(std::optional<std::int32_t> initial, std::set<std::int32_t> values) : value_(initial.value_or(0)), values_(std::move(values)) {}
    std::int32_t getValue() const noexcept override { return value_; }
    void setValue(std::int32_t value) noexcept override { value_ = value; }
    std::string getTitle() const noexcept override { return title_; }
    void setTitle(std::string value) noexcept override { title_ = std::move(value); }
    std::vector<std::uint8_t> getPayload() const noexcept override { return payload_; }
    void setPayload(std::vector<std::uint8_t> value) noexcept override { payload_ = std::move(value); }
    std::optional<std::string> getMaybeTitle() const noexcept override { return maybe_title_; }
    void setMaybeTitle(std::optional<std::string> value) noexcept override { maybe_title_ = std::move(value); }
    std::optional<std::vector<std::uint8_t>> getMaybePayload() const noexcept override { return maybe_payload_; }
    void setMaybePayload(std::optional<std::vector<std::uint8_t>> value) noexcept override { maybe_payload_ = std::move(value); }
    std::vector<std::int32_t> getSamples() const noexcept override { return samples_; }
    void setSamples(std::vector<std::int32_t> value) noexcept override { samples_ = std::move(value); }
    std::set<std::int32_t> getValues() const noexcept override { return values_; }
    void setValues(std::set<std::int32_t> value) noexcept override { values_ = std::move(value); }
    std::string echo(std::string value) noexcept override { return value; }
    std::vector<std::uint8_t> echoBytes(std::vector<std::uint8_t> value) noexcept override { return value; }
    std::optional<std::string> maybeEcho(std::optional<std::string> value) noexcept override { return value; }
    std::vector<std::int32_t> echoValues(std::vector<std::int32_t> values) noexcept override { return values; }
    std::set<std::int32_t> echoSet(std::set<std::int32_t> values) noexcept override { return values; }
    void increment() noexcept override { ++value_; }
    void setOnChanged(std::function<void(std::int32_t, std::string, std::vector<std::uint8_t>)> handler) noexcept override { on_changed_ = std::move(handler); }
    void notify(std::int32_t value) noexcept override { if (on_changed_) on_changed_(value, title_, payload_); }
private:
    std::int32_t value_;
    std::string title_;
    std::vector<std::uint8_t> payload_;
    std::optional<std::string> maybe_title_;
    std::optional<std::vector<std::uint8_t>> maybe_payload_;
    std::vector<std::int32_t> samples_;
    std::set<std::int32_t> values_;
    std::function<void(std::int32_t, std::string, std::vector<std::uint8_t>)> on_changed_;
};
std::unique_ptr<CounterValueSpec> makeCounterValueImpl(std::optional<std::int32_t> initial, std::set<std::int32_t> values) {
    return std::make_unique<CounterValueImpl>(initial, std::move(values));
}
}
int nexa_plugin_probe() { return details::value(); }
"#,
    )
    .expect("C++ plugin source should be written");
    let entry = temp.0.join("main.nx");
    fs::write(
        &entry,
        "plugin \"cpp-plugin\" as Counter\napp Demo { body { Button(\"Ping\") { Counter.ping() } } }\n",
    )
    .expect("Nexa app should be written");
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "ios", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("CppPluginBuildTest")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "iOS project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );
    let project =
        fs::read_to_string(output.join("ios/CppPluginBuildTest.xcodeproj/project.pbxproj"))
            .expect("generated Xcode project should be readable");
    assert!(project.contains("lastKnownFileType = sourcecode.cpp.cpp"));
    assert!(project.contains("NexaPluginCpp/Plugin0/cpp/Sources/Probe.cpp"));
    assert!(project.contains("HEADER_SEARCH_PATHS"));
    assert!(project.contains("CLANG_CXX_LANGUAGE_STANDARD = \"c++23\""));
    assert!(project.contains("SWIFT_OBJC_INTEROP_MODE = objcxx"));
    assert!(
        output
            .join("ios/CppPluginBuildTest/NexaPluginCpp/Plugin0/NexaPluginBindings.hpp")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(output.join("ios/CppPluginBuildTest/NexaPluginCpp-Bridging-Header.h"))
            .expect("generated C++ bridging header should be readable"),
        "#include \"NexaPluginCpp/Plugin0/NexaPluginBindings.hpp\"\n"
    );
    if command_available("xcrun", &["--sdk", "iphonesimulator", "--show-sdk-path"])
        && command_available("xcodebuild", &["-version"])
    {
        let built = Command::new("xcodebuild")
            .args([
                "-quiet",
                "-project",
                "ios/CppPluginBuildTest.xcodeproj",
                "-scheme",
                "CppPluginBuildTest",
                "-sdk",
                "iphonesimulator",
                "-destination",
                "generic/platform=iOS Simulator",
                "-derivedDataPath",
            ])
            .arg(temp.0.join("CppDerivedData"))
            .arg("CODE_SIGNING_ALLOWED=NO")
            .arg("build")
            .current_dir(&output)
            .output()
            .expect("xcodebuild should start when its simulator SDK is available");
        assert!(
            built.status.success(),
            "generated iOS C++ plugin source failed to build:\n{}\n{}",
            String::from_utf8_lossy(&built.stdout),
            String::from_utf8_lossy(&built.stderr)
        );
    }
}

#[test]
fn generated_android_cpp_plugin_uses_jni_adapters_and_compiles_with_ndk_when_available() {
    let temp = TempProject::new("android-cpp-jni");
    let plugin = temp.0.join("cpp-plugin");
    fs::create_dir_all(plugin.join("cpp/Sources"))
        .expect("C++ plugin source directory should be created");
    fs::write(
        plugin.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.android-cpp"
    version: "1.0.0"
    sources { native: "native.nxid" }
    cpp { standard: "c++17" sources: ["cpp/Sources/**"] }
}
"#,
    )
    .expect("plugin manifest should be written");
    fs::write(
        plugin.join("native.nxid"),
        r#"error LookupError {
    missing
    malformed(code: UInt32, message: String, payload: Bytes)
}
service Counter {
    fn increment(value: Int32) -> Int32
    async fn incrementAsync(value: Int32) -> Int32
    async fn maybeAsync(value: Int32?) -> Int32?
    async fn echoTextAsync(value: String) -> String
    async fn echoBytesAsync(value: Bytes) -> Bytes
    async fn pingAsync()
    async fn read(value: Bool) throws LookupError
    async fn readValue(value: Int32) -> Result<Int32, LookupError>
    async fn echoAsyncIntegers(values: Array<Int32>) -> Array<Int32>
    async fn echoAsyncMap(values: Map<String, Int32>) -> Map<String, Int32>
    fn echoIntegers(values: Array<Int32>) -> Array<Int32>
    fn echoUnsigned(values: Array<UInt32>) -> Array<UInt32>
    fn echoDoubles(values: Array<Float64>) -> Array<Float64>
    fn echoStrings(values: Array<String>) -> Array<String>
    fn echoByteArrays(values: Array<Bytes>) -> Array<Bytes>
    fn echoNestedIntegers(values: Array<Array<Int32>>) -> Array<Array<Int32>>
    fn echoNestedBooleans(values: Array<Array<Bool>>) -> Array<Array<Bool>>
    fn echoNestedStrings(values: Array<Array<String>>) -> Array<Array<String>>
    fn echoNestedPayloads(values: Array<Array<Bytes>>) -> Array<Array<Bytes>>
    fn echoUIntSet(values: Set<UInt32>) -> Set<UInt32>
    fn echoIntSet(values: Set<Int32>) -> Set<Int32>
    fn echoStringSet(values: Set<String>) -> Set<String>
    fn echoMap(values: Map<String, Int32>) -> Map<String, Int32>
    fn echoUnsignedMap(values: Map<UInt32, UInt64>) -> Map<UInt32, UInt64>
    fn echoFloatMap(values: Map<String, Float64>) -> Map<String, Float64>
    fn echoBooleanMap(values: Map<Bool, Float32>) -> Map<Bool, Float32>
    fn echoText(value: String) -> String
    fn echoBytes(value: Bytes) -> Bytes
    fn echoUByte(value: UInt8) -> UInt8
    fn echoUShort(value: UInt16) -> UInt16
    fn echoUInt(value: UInt32) -> UInt32
    fn echoULong(value: UInt64) -> UInt64
    fn maybeInt(value: Int32?) -> Int32?
    fn maybeBool(value: Bool?) -> Bool?
    fn maybeByte(value: Int8?) -> Int8?
    fn maybeShort(value: Int16?) -> Int16?
    fn maybeFloat(value: Float32?) -> Float32?
    fn maybeDouble(value: Float64?) -> Float64?
    fn maybeUByte(value: UInt8?) -> UInt8?
    fn maybeUShort(value: UInt16?) -> UInt16?
    fn maybeUInt(value: UInt32?) -> UInt32?
    fn maybeULong(value: UInt64?) -> UInt64?
    fn maybeText(value: String?) -> String?
    fn maybeBytes(value: Bytes?) -> Bytes?
    fn ping()
    fn disposeCount() -> Int32
}
native class Meter {
    init(initial: Float64, label: String, initialCount: Int32?, metrics: Map<String, Int32>)
    event changed(value: Int32, label: String, payload: Bytes)
    property value: Float64
    property label: String
    property payload: Bytes
    property sampleValues: Array<Int32>
    property unsignedValue: UInt32
    property maybeCount: Int32?
    property maybeLabel: String?
    property maybePayload: Bytes?
    property metrics: Map<String, Int32>
    fn add(amount: Float64) -> Float64
    async fn validate(value: Bool) throws LookupError
    fn echo(value: String) -> String
    fn echoBytes(value: Bytes) -> Bytes
    fn echoUnsigned(value: UInt64) -> UInt64
    fn echoValues(values: Array<Int32>) -> Array<Int32>
    fn maybeNumber(value: Int64?) -> Int64?
    fn echoMetrics(values: Map<String, Int32>) -> Map<String, Int32>
    async fn currentValue() -> Float64
    fn publish(value: Int32)
    fn clear()
    fn dispose()
}
"#,
    )
    .expect("native contract should be written");
    fs::write(
        plugin.join("cpp/Sources/Plugin.cpp"),
        r#"#include "NexaPluginBindings.hpp"
#include <atomic>
#include <memory>
#include <future>
#include <string>
#include <utility>
#include <vector>

namespace plugin_dev::plugin_example::plugin_android_dash_cpp {
namespace { std::atomic<std::int32_t> destroyed_meters{0}; }
template<class T> std::future<T> readyFuture(T value) noexcept {
    std::promise<T> promise;
    promise.set_value(std::move(value));
    return promise.get_future();
}
std::future<void> readyFuture() noexcept {
    std::promise<void> promise;
    promise.set_value();
    return promise.get_future();
}
namespace Counter {
std::int32_t increment(std::int32_t value) noexcept { return value + 1; }
std::future<std::int32_t> incrementAsync(std::int32_t value) noexcept { return readyFuture(value + 1); }
std::future<std::optional<std::int32_t>> maybeAsync(std::optional<std::int32_t> value) noexcept { return readyFuture(value); }
std::future<std::string> echoTextAsync(std::string value) noexcept { return readyFuture(std::move(value)); }
std::future<std::vector<std::uint8_t>> echoBytesAsync(std::vector<std::uint8_t> value) noexcept { return readyFuture(std::move(value)); }
std::future<void> pingAsync() noexcept { return readyFuture(); }
std::future<NexaResult<void, LookupError>> read(bool value) noexcept {
    if (!value) return readyFuture(NexaResult<void, LookupError>::success());
    return readyFuture(NexaResult<void, LookupError>::failure(
        LookupError{LookupError::Value{LookupError::MalformedCase{UINT32_MAX, "bad input", {0, 255}}}}));
}
std::future<NexaResult<std::int32_t, LookupError>> readValue(std::int32_t value) noexcept {
    if (value >= 0) return readyFuture(NexaResult<std::int32_t, LookupError>::success(value));
    return readyFuture(NexaResult<std::int32_t, LookupError>::failure(
        LookupError{LookupError::Value{LookupError::MissingCase{}}}));
}
std::future<std::vector<std::int32_t>> echoAsyncIntegers(std::vector<std::int32_t> values) noexcept {
    return readyFuture(std::move(values));
}
std::future<std::map<std::string, std::int32_t>> echoAsyncMap(std::map<std::string, std::int32_t> values) noexcept {
    return readyFuture(std::move(values));
}
std::vector<std::int32_t> echoIntegers(std::vector<std::int32_t> values) noexcept { return values; }
std::vector<std::uint32_t> echoUnsigned(std::vector<std::uint32_t> values) noexcept { return values; }
std::vector<double> echoDoubles(std::vector<double> values) noexcept { return values; }
std::vector<std::string> echoStrings(std::vector<std::string> values) noexcept { return values; }
std::vector<std::vector<std::uint8_t>> echoByteArrays(std::vector<std::vector<std::uint8_t>> values) noexcept { return values; }
std::vector<std::vector<std::int32_t>> echoNestedIntegers(std::vector<std::vector<std::int32_t>> values) noexcept { return values; }
std::vector<std::vector<bool>> echoNestedBooleans(std::vector<std::vector<bool>> values) noexcept { return values; }
std::vector<std::vector<std::string>> echoNestedStrings(std::vector<std::vector<std::string>> values) noexcept { return values; }
std::vector<std::vector<std::vector<std::uint8_t>>> echoNestedPayloads(std::vector<std::vector<std::vector<std::uint8_t>>> values) noexcept { return values; }
std::set<std::uint32_t> echoUIntSet(std::set<std::uint32_t> values) noexcept { return values; }
std::set<std::int32_t> echoIntSet(std::set<std::int32_t> values) noexcept { return values; }
std::set<std::string> echoStringSet(std::set<std::string> values) noexcept { return values; }
std::map<std::string, std::int32_t> echoMap(std::map<std::string, std::int32_t> values) noexcept { return values; }
std::map<std::uint32_t, std::uint64_t> echoUnsignedMap(std::map<std::uint32_t, std::uint64_t> values) noexcept { return values; }
std::map<std::string, double> echoFloatMap(std::map<std::string, double> values) noexcept { return values; }
std::map<bool, float> echoBooleanMap(std::map<bool, float> values) noexcept { return values; }
std::string echoText(std::string value) noexcept { return value; }
std::vector<std::uint8_t> echoBytes(std::vector<std::uint8_t> value) noexcept { return value; }
std::uint8_t echoUByte(std::uint8_t value) noexcept { return value; }
std::uint16_t echoUShort(std::uint16_t value) noexcept { return value; }
std::uint32_t echoUInt(std::uint32_t value) noexcept { return value; }
std::uint64_t echoULong(std::uint64_t value) noexcept { return value; }
std::optional<std::int32_t> maybeInt(std::optional<std::int32_t> value) noexcept { return value; }
std::optional<bool> maybeBool(std::optional<bool> value) noexcept { return value; }
std::optional<std::int8_t> maybeByte(std::optional<std::int8_t> value) noexcept { return value; }
std::optional<std::int16_t> maybeShort(std::optional<std::int16_t> value) noexcept { return value; }
std::optional<float> maybeFloat(std::optional<float> value) noexcept { return value; }
std::optional<double> maybeDouble(std::optional<double> value) noexcept { return value; }
std::optional<std::uint8_t> maybeUByte(std::optional<std::uint8_t> value) noexcept { return value; }
std::optional<std::uint16_t> maybeUShort(std::optional<std::uint16_t> value) noexcept { return value; }
std::optional<std::uint32_t> maybeUInt(std::optional<std::uint32_t> value) noexcept { return value; }
std::optional<std::uint64_t> maybeULong(std::optional<std::uint64_t> value) noexcept { return value; }
std::optional<std::string> maybeText(std::optional<std::string> value) noexcept { return value; }
std::optional<std::vector<std::uint8_t>> maybeBytes(std::optional<std::vector<std::uint8_t>> value) noexcept { return value; }
void ping() noexcept {}
std::int32_t disposeCount() noexcept { return destroyed_meters.load(); }
}
class MeterImpl final : public MeterSpec {
public:
    MeterImpl(double initial, std::string label, std::optional<std::int32_t> initial_count, std::map<std::string, std::int32_t> metrics)
        : value_(initial), label_(std::move(label)), maybe_count_(initial_count), metrics_(std::move(metrics)) {}
    ~MeterImpl() override { ++destroyed_meters; }
    double getValue() const noexcept override { return value_; }
    void setValue(double value) noexcept override { value_ = value; }
    std::string getLabel() const noexcept override { return label_; }
    void setLabel(std::string value) noexcept override { label_ = std::move(value); }
    std::vector<std::uint8_t> getPayload() const noexcept override { return payload_; }
    void setPayload(std::vector<std::uint8_t> value) noexcept override { payload_ = std::move(value); }
    std::vector<std::int32_t> getSampleValues() const noexcept override { return sample_values_; }
    void setSampleValues(std::vector<std::int32_t> value) noexcept override { sample_values_ = std::move(value); }
    std::uint32_t getUnsignedValue() const noexcept override { return unsigned_value_; }
    void setUnsignedValue(std::uint32_t value) noexcept override { unsigned_value_ = value; }
    std::optional<std::int32_t> getMaybeCount() const noexcept override { return maybe_count_; }
    void setMaybeCount(std::optional<std::int32_t> value) noexcept override { maybe_count_ = value; }
    std::optional<std::string> getMaybeLabel() const noexcept override { return maybe_label_; }
    void setMaybeLabel(std::optional<std::string> value) noexcept override { maybe_label_ = std::move(value); }
    std::optional<std::vector<std::uint8_t>> getMaybePayload() const noexcept override { return maybe_payload_; }
    void setMaybePayload(std::optional<std::vector<std::uint8_t>> value) noexcept override { maybe_payload_ = std::move(value); }
    std::map<std::string, std::int32_t> getMetrics() const noexcept override { return metrics_; }
    void setMetrics(std::map<std::string, std::int32_t> value) noexcept override { metrics_ = std::move(value); }
    double add(double amount) noexcept override { value_ += amount; return value_; }
    std::future<NexaResult<void, LookupError>> validate(bool value) noexcept override {
        if (!value) return readyFuture(NexaResult<void, LookupError>::success());
        return readyFuture(NexaResult<void, LookupError>::failure(
            LookupError{LookupError::Value{LookupError::MissingCase{}}}));
    }
    std::string echo(std::string value) noexcept override { return value; }
    std::vector<std::uint8_t> echoBytes(std::vector<std::uint8_t> value) noexcept override { return value; }
    std::uint64_t echoUnsigned(std::uint64_t value) noexcept override { return value; }
    std::vector<std::int32_t> echoValues(std::vector<std::int32_t> values) noexcept override { return values; }
    std::optional<std::int64_t> maybeNumber(std::optional<std::int64_t> value) noexcept override { return value; }
    std::map<std::string, std::int32_t> echoMetrics(std::map<std::string, std::int32_t> values) noexcept override { return values; }
    std::future<double> currentValue() noexcept override { return readyFuture(value_); }
    void setOnChanged(std::function<void(std::int32_t, std::string, std::vector<std::uint8_t>)> handler) noexcept override { on_changed_ = std::move(handler); }
    void publish(std::int32_t value) noexcept override {
        if (!on_changed_) return;
        std::thread event_thread([this, value] { on_changed_(value, label_, payload_); });
        event_thread.join();
    }
    void clear() noexcept override { value_ = 0; }
    void dispose() noexcept override {}
private:
    double value_;
    std::string label_;
    std::vector<std::uint8_t> payload_;
    std::vector<std::int32_t> sample_values_;
    std::uint32_t unsigned_value_{};
    std::optional<std::int32_t> maybe_count_;
    std::optional<std::string> maybe_label_;
    std::optional<std::vector<std::uint8_t>> maybe_payload_;
    std::map<std::string, std::int32_t> metrics_;
    std::function<void(std::int32_t, std::string, std::vector<std::uint8_t>)> on_changed_;
};
std::unique_ptr<MeterSpec> makeMeterImpl(double initial, std::string label, std::optional<std::int32_t> initial_count, std::map<std::string, std::int32_t> metrics) {
    return std::make_unique<MeterImpl>(initial, std::move(label), initial_count, std::move(metrics));
}
}
"#,
    )
    .expect("C++ implementation should be written");
    let entry = temp.0.join("main.nx");
    fs::write(
        &entry,
        "plugin \"cpp-plugin\" as Counter\napp Demo { body { Button(\"Increment\") { Counter.increment(value: 1) } } }\n",
    )
    .expect("Nexa app should be written");

    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "android", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("AndroidCppJniTest")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "Android project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let package = output.join("android/app/src/main/java/com/nexa/androidcppjnitest");
    let bindings = fs::read_to_string(package.join("NexaPlugin0_Bindings.kt"))
        .expect("generated Kotlin plugin bindings should be present");
    assert!(bindings.contains("external fun service_Counter_increment(value: Int): Int"));
    assert!(bindings.contains("external fun service_Counter_incrementAsync(value: Int): Int"));
    assert!(bindings.contains("override suspend fun incrementAsync(value: Int): Int"));
    assert!(bindings.contains("override suspend fun maybeAsync(value: Int?): Int?"));
    assert!(bindings.contains("override suspend fun echoTextAsync(value: String): String"));
    assert!(bindings.contains("override suspend fun echoBytesAsync(value: ByteArray): ByteArray"));
    assert!(bindings.contains("override suspend fun pingAsync(): Unit"));
    assert!(bindings.contains("@Throws(LookupError::class)"));
    assert!(bindings.contains("override suspend fun read(value: Boolean): Unit"));
    assert!(bindings.contains("override suspend fun readValue(value: Int): Int"));
    assert!(
        bindings.contains("override suspend fun echoAsyncIntegers(values: List<Int>): List<Int>")
    );
    assert!(
        bindings.contains(
            "override suspend fun echoAsyncMap(values: Map<String, Int>): Map<String, Int>"
        )
    );
    assert!(bindings.contains("override suspend fun validate(value: Boolean): Unit"));
    assert!(bindings.contains("internal object NexaPlugin0_CppErrorFactory"));
    assert!(bindings.contains(
        "createLookupErrorCase1(code: Int, message: String, payload: ByteArray): LookupError"
    ));
    assert!(bindings.contains("kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO)"));
    assert!(bindings.contains(
        "override suspend fun currentValue(): Double = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) { synchronized(this) { NexaPlugin0_CppBindings.call_Meter_currentValue(requireNativeHandle()) } }"
    ));
    assert!(
        bindings.contains("external fun service_Counter_echoIntegers(values: IntArray): IntArray")
    );
    assert!(
        bindings.contains("external fun service_Counter_echoUnsigned(values: IntArray): IntArray")
    );
    assert!(
        bindings
            .contains("external fun service_Counter_echoDoubles(values: DoubleArray): DoubleArray")
    );
    assert!(bindings.contains(
        "external fun service_Counter_echoStrings(values: Array<String>): Array<String>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoByteArrays(values: Array<ByteArray>): Array<ByteArray>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoNestedIntegers(values: Array<IntArray>): Array<IntArray>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoNestedBooleans(values: Array<BooleanArray>): Array<BooleanArray>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoNestedStrings(values: Array<Array<String>>): Array<Array<String>>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoNestedPayloads(values: Array<Array<ByteArray>>): Array<Array<ByteArray>>"
    ));
    assert!(
        bindings.contains("external fun service_Counter_echoUIntSet(values: IntArray): IntArray")
    );
    assert!(bindings.contains(
        "external fun service_Counter_echoStringSet(values: Array<String>): Array<String>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoMap(values: Map<String, Int>): Map<String, Int>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoUnsignedMap(values: Map<Int, Long>): Map<Int, Long>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoFloatMap(values: Map<String, Double>): Map<String, Double>"
    ));
    assert!(bindings.contains(
        "external fun service_Counter_echoBooleanMap(values: Map<Boolean, Float>): Map<Boolean, Float>"
    ));
    assert!(bindings.contains("external fun service_Counter_echoUByte(value: Byte): Byte"));
    assert!(bindings.contains("external fun service_Counter_echoUShort(value: Short): Short"));
    assert!(bindings.contains("external fun service_Counter_echoUInt(value: Int): Int"));
    assert!(bindings.contains("external fun service_Counter_echoULong(value: Long): Long"));
    assert!(bindings.contains("override fun echoUByte(value: UByte): UByte"));
    assert!(bindings.contains("public object CounterPlugin : Counter"));
    assert!(bindings.contains("public val instance: CounterPlugin get() = this"));
    assert!(bindings.contains("public class MeterImpl"));
    assert!(bindings.contains("override var onChanged: ((Int, String, ByteArray) -> Unit)?"));
    assert!(bindings.contains("@JvmName(\"nexaDispatch\")"));
    assert!(bindings.contains("NexaPlugin0_CppBindings.dispose_Meter(handle)"));
    assert!(bindings.contains("System.loadLibrary(\"nexa_plugins\")"));
    let gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("generated Android Gradle configuration should be readable");
    assert!(
        gradle
            .contains("implementation(\"org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0\")")
    );
    let jni = fs::read_to_string(output.join("android/app/src/main/cpp/Plugin0/NexaPluginJni.cpp"))
        .expect("generated C++ JNI adapters should be readable");
    assert!(jni.contains("auto nexaCppFuture = Counter::incrementAsync"));
    assert!(jni.contains("nexaCppFuture.get()"));
    assert!(jni.contains("nexaCppThrowErrorLookupError(env, nexaCppTypedResult.error())"));

    let kotlinc = kotlin_compiler();
    let kotlin_jar = temp.0.join("NexaPluginBindings.jar");
    if let Some(kotlinc) = &kotlinc {
        let smoke_test = package.join("AndroidCppSmoke.kt");
        fs::write(
            &smoke_test,
            r#"package com.nexa.androidcppjnitest

import kotlinx.coroutines.runBlocking
import java.util.concurrent.atomic.AtomicReference

fun main() = runBlocking {
    check(CounterPlugin.instance.increment(41) == 42)
    check(CounterPlugin.instance.incrementAsync(41) == 42)
    check(CounterPlugin.instance.maybeAsync(42) == 42)
    check(CounterPlugin.instance.maybeAsync(null) == null)
    val asyncText = "Nexa\u0000 🚀"
    check(CounterPlugin.instance.echoTextAsync(asyncText) == asyncText)
    val asyncBytes = byteArrayOf(0, -1, 42)
    check(CounterPlugin.instance.echoBytesAsync(asyncBytes).contentEquals(asyncBytes))
    val asyncValues = listOf(Int.MIN_VALUE, 0, Int.MAX_VALUE)
    check(CounterPlugin.instance.echoAsyncIntegers(asyncValues) == asyncValues)
    val asyncMap = mapOf("Nexa\u0000 🚀" to Int.MIN_VALUE, "last" to Int.MAX_VALUE)
    check(CounterPlugin.instance.echoAsyncMap(asyncMap) == asyncMap)
    CounterPlugin.instance.pingAsync()
    CounterPlugin.instance.read(false)
    val malformed = try {
        CounterPlugin.instance.read(true)
        error("read(true) should throw the typed error")
    } catch (error: LookupError.malformed) {
        error
    }
    check(malformed.code == UInt.MAX_VALUE)
    check(malformed.message == "bad input")
    check(malformed.payload.contentEquals(byteArrayOf(0, -1)))
    check(CounterPlugin.instance.readValue(42) == 42)
    val missing: LookupError? = try {
        CounterPlugin.instance.readValue(-1)
        error("readValue(-1) should throw the typed error")
    } catch (error: LookupError) {
        error
    }
    check(missing === LookupError.missing)
    check(CounterPlugin.instance.echoIntegers(listOf(Int.MIN_VALUE, -1, 0, 42, Int.MAX_VALUE)) == listOf(Int.MIN_VALUE, -1, 0, 42, Int.MAX_VALUE))
    check(CounterPlugin.instance.echoIntegers(emptyList()).isEmpty())
    check(CounterPlugin.instance.echoUnsigned(listOf(0u, 1u, UInt.MAX_VALUE)) == listOf(0u, 1u, UInt.MAX_VALUE))
    check(CounterPlugin.instance.echoDoubles(listOf(-1.25, 0.0, Double.MAX_VALUE)) == listOf(-1.25, 0.0, Double.MAX_VALUE))
    check(CounterPlugin.instance.echoDoubles(emptyList()).isEmpty())
    val strings = listOf("", "Nexa\u0000 🚀", "last")
    check(CounterPlugin.instance.echoStrings(strings) == strings)
    check(CounterPlugin.instance.echoStrings(emptyList()).isEmpty())
    val byteArrays = listOf(byteArrayOf(0, -1, 42), byteArrayOf(), ByteArray(256) { it.toByte() })
    val returnedByteArrays = CounterPlugin.instance.echoByteArrays(byteArrays)
    check(returnedByteArrays.size == byteArrays.size)
    check(returnedByteArrays.indices.all { returnedByteArrays[it].contentEquals(byteArrays[it]) })
    check(CounterPlugin.instance.echoByteArrays(emptyList()).isEmpty())
    val nestedIntegers = listOf(listOf(Int.MIN_VALUE, 0, Int.MAX_VALUE), emptyList(), listOf(-1, 7))
    check(CounterPlugin.instance.echoNestedIntegers(nestedIntegers) == nestedIntegers)
    check(CounterPlugin.instance.echoNestedIntegers(emptyList()).isEmpty())
    val nestedBooleans = listOf(listOf(false, true), emptyList(), listOf(true))
    check(CounterPlugin.instance.echoNestedBooleans(nestedBooleans) == nestedBooleans)
    check(CounterPlugin.instance.echoNestedBooleans(emptyList()).isEmpty())
    val nestedStrings = listOf(listOf("Nexa\u0000 🚀", ""), emptyList(), listOf("last"))
    check(CounterPlugin.instance.echoNestedStrings(nestedStrings) == nestedStrings)
    val nestedPayloads = listOf(listOf(byteArrayOf(0, -1), byteArrayOf()), emptyList(), listOf(byteArrayOf(42)))
    val returnedNestedPayloads = CounterPlugin.instance.echoNestedPayloads(nestedPayloads)
    check(returnedNestedPayloads.size == nestedPayloads.size)
    check(returnedNestedPayloads.indices.all { row ->
        returnedNestedPayloads[row].size == nestedPayloads[row].size &&
            returnedNestedPayloads[row].indices.all { column ->
                returnedNestedPayloads[row][column].contentEquals(nestedPayloads[row][column])
            }
    })
    check(CounterPlugin.instance.echoNestedPayloads(emptyList()).isEmpty())
    val unsignedSet = linkedSetOf(0u, UInt.MAX_VALUE, 1u)
    check(CounterPlugin.instance.echoUIntSet(unsignedSet) == unsignedSet)
    check(CounterPlugin.instance.echoUIntSet(emptySet()).isEmpty())
    val signedSet = setOf(Int.MIN_VALUE, -1, 0, Int.MAX_VALUE)
    check(CounterPlugin.instance.echoIntSet(signedSet) == signedSet)
    check(CounterPlugin.instance.echoIntSet(emptySet()).isEmpty())
    val stringSet = setOf("", "Nexa\u0000 🚀", "last")
    check(CounterPlugin.instance.echoStringSet(stringSet) == stringSet)
    check(CounterPlugin.instance.echoStringSet(emptySet()).isEmpty())
    val stringMap = mapOf("" to Int.MIN_VALUE, "Nexa\u0000 🚀" to 42)
    check(CounterPlugin.instance.echoMap(stringMap) == stringMap)
    check(CounterPlugin.instance.echoMap(emptyMap()).isEmpty())
    val unsignedMap = mapOf(0u to 0uL, UInt.MAX_VALUE to ULong.MAX_VALUE)
    check(CounterPlugin.instance.echoUnsignedMap(unsignedMap) == unsignedMap)
    val floatMap = mapOf("low" to -1.25, "high" to Double.MAX_VALUE)
    check(CounterPlugin.instance.echoFloatMap(floatMap) == floatMap)
    val booleanMap = mapOf(false to -1.25f, true to 3.5f)
    check(CounterPlugin.instance.echoBooleanMap(booleanMap) == booleanMap)
    check(CounterPlugin.instance.echoUByte(UByte.MAX_VALUE) == UByte.MAX_VALUE)
    check(CounterPlugin.instance.echoUShort(UShort.MAX_VALUE) == UShort.MAX_VALUE)
    check(CounterPlugin.instance.echoUInt(UInt.MAX_VALUE) == UInt.MAX_VALUE)
    check(CounterPlugin.instance.echoULong(ULong.MAX_VALUE) == ULong.MAX_VALUE)
    check(CounterPlugin.instance.maybeInt(37) == 37)
    check(CounterPlugin.instance.maybeInt(null) == null)
    check(CounterPlugin.instance.maybeBool(true) == true)
    check(CounterPlugin.instance.maybeBool(null) == null)
    check(CounterPlugin.instance.maybeByte(Byte.MIN_VALUE) == Byte.MIN_VALUE)
    check(CounterPlugin.instance.maybeByte(null) == null)
    check(CounterPlugin.instance.maybeShort(Short.MAX_VALUE) == Short.MAX_VALUE)
    check(CounterPlugin.instance.maybeShort(null) == null)
    check(CounterPlugin.instance.maybeFloat(1.25f) == 1.25f)
    check(CounterPlugin.instance.maybeFloat(null) == null)
    check(CounterPlugin.instance.maybeDouble(9.5) == 9.5)
    check(CounterPlugin.instance.maybeDouble(null) == null)
    check(CounterPlugin.instance.maybeUByte(UByte.MAX_VALUE) == UByte.MAX_VALUE)
    check(CounterPlugin.instance.maybeUByte(null) == null)
    check(CounterPlugin.instance.maybeUShort(UShort.MAX_VALUE) == UShort.MAX_VALUE)
    check(CounterPlugin.instance.maybeUShort(null) == null)
    check(CounterPlugin.instance.maybeUInt(UInt.MAX_VALUE) == UInt.MAX_VALUE)
    check(CounterPlugin.instance.maybeUInt(null) == null)
    check(CounterPlugin.instance.maybeULong(ULong.MAX_VALUE) == ULong.MAX_VALUE)
    check(CounterPlugin.instance.maybeULong(null) == null)
    val embeddedNullAndUnicode = "Nexa\u0000 🚀"
    check(CounterPlugin.instance.echoText(embeddedNullAndUnicode) == embeddedNullAndUnicode)
    check(CounterPlugin.instance.maybeText(embeddedNullAndUnicode) == embeddedNullAndUnicode)
    check(CounterPlugin.instance.maybeText(null) == null)
    val binaryPayload = byteArrayOf(0, -1, 0x7f, -128) + ByteArray(1024) { it.toByte() }
    check(CounterPlugin.instance.echoBytes(binaryPayload).contentEquals(binaryPayload))
    check(CounterPlugin.instance.echoBytes(byteArrayOf()).isEmpty())
    check(CounterPlugin.instance.maybeBytes(binaryPayload)!!.contentEquals(binaryPayload))
    check(CounterPlugin.instance.maybeBytes(byteArrayOf())!!.isEmpty())
    check(CounterPlugin.instance.maybeBytes(null) == null)
    CounterPlugin.instance.ping()
    check(CounterPlugin.instance.disposeCount() == 0)

    val initialMetrics = mapOf("first" to 1, "second" to Int.MAX_VALUE)
    val first = Meter(2.5, embeddedNullAndUnicode, 13, initialMetrics)
    val second = Meter(11.0, "second", null, emptyMap())
    val firstEvent = AtomicReference<Triple<Int, String, ByteArray>?>(null)
    val secondEvent = AtomicReference<Triple<Int, String, ByteArray>?>(null)
    first.onChanged = { value, label, payload -> firstEvent.set(Triple(value, label, payload)) }
    second.onChanged = { value, label, payload -> secondEvent.set(Triple(value, label, payload)) }
    first.payload = binaryPayload
    first.publish(42)
    second.publish(-7)
    check(firstEvent.get()?.first == 42 && firstEvent.get()?.second == embeddedNullAndUnicode)
    check(firstEvent.get()?.third?.contentEquals(binaryPayload) == true)
    check(secondEvent.get()?.first == -7 && secondEvent.get()?.second == "second")
    check(secondEvent.get()?.third?.isEmpty() == true)
    first.onChanged = null
    firstEvent.set(null)
    first.publish(43)
    check(firstEvent.get() == null)
    check(first.value == 2.5)
    check(first.currentValue() == 2.5)
    check(first.label == embeddedNullAndUnicode)
    check(first.metrics == initialMetrics)
    first.metrics = mapOf("updated" to Int.MIN_VALUE)
    check(first.metrics == mapOf("updated" to Int.MIN_VALUE))
    check(first.echoMetrics(initialMetrics) == initialMetrics)
    check(first.maybeCount == 13)
    check(second.maybeCount == null)
    first.maybeCount = null
    check(first.maybeCount == null)
    first.maybeCount = 21
    check(first.maybeCount == 21)
    first.maybeLabel = embeddedNullAndUnicode
    check(first.maybeLabel == embeddedNullAndUnicode)
    first.maybeLabel = null
    check(first.maybeLabel == null)
    first.label = "changed\u0000 🧪"
    check(first.label == "changed\u0000 🧪")
    check(first.echo(embeddedNullAndUnicode) == embeddedNullAndUnicode)
    first.payload = binaryPayload
    check(first.payload.contentEquals(binaryPayload))
    first.sampleValues = listOf(Int.MIN_VALUE, 4, Int.MAX_VALUE)
    check(first.sampleValues == listOf(Int.MIN_VALUE, 4, Int.MAX_VALUE))
    check(first.echoValues(listOf(-1, 0, 7)) == listOf(-1, 0, 7))
    check(first.echoValues(emptyList()).isEmpty())
    first.maybePayload = binaryPayload
    check(first.maybePayload!!.contentEquals(binaryPayload))
    first.maybePayload = null
    check(first.maybePayload == null)
    check(first.echoBytes(binaryPayload).contentEquals(binaryPayload))
    first.unsignedValue = UInt.MAX_VALUE
    check(first.unsignedValue == UInt.MAX_VALUE)
    check(first.echoUnsigned(ULong.MAX_VALUE) == ULong.MAX_VALUE)
    check(first.maybeNumber(Long.MAX_VALUE) == Long.MAX_VALUE)
    check(first.maybeNumber(null) == null)
    check(second.label == "second")
    first.value = 4.0
    check(first.add(3.0) == 7.0)
    first.validate(false)
    val classError: LookupError? = try {
        first.validate(true)
        error("validate(true) should throw the typed error")
    } catch (error: LookupError) {
        error
    }
    check(classError === LookupError.missing)
    check(second.value == 11.0)
    first.clear()
    check(first.value == 0.0)
    check(second.value == 11.0)

    first.dispose()
    check(CounterPlugin.instance.disposeCount() == 1)
    first.dispose()
    check(CounterPlugin.instance.disposeCount() == 1)
    check(runCatching { first.add(1.0) }.exceptionOrNull() is IllegalStateException)
    second.dispose()
    check(CounterPlugin.instance.disposeCount() == 2)
}
"#,
        )
        .expect("JNI runtime smoke test should be written");
        let coroutine_support = temp.0.join("CoroutineSupport.kt");
        fs::write(
            &coroutine_support,
            r#"package kotlinx.coroutines

import kotlin.coroutines.Continuation
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.EmptyCoroutineContext
import kotlin.coroutines.startCoroutine

object Dispatchers {
    val IO: CoroutineContext = EmptyCoroutineContext
}

suspend fun <T> withContext(context: CoroutineContext, block: suspend () -> T): T = block()

fun <T> runBlocking(block: suspend () -> T): T {
    var value: Any? = null
    var failure: Throwable? = null
    block.startCoroutine(object : Continuation<T> {
        override val context: CoroutineContext = EmptyCoroutineContext
        override fun resumeWith(result: Result<T>) {
            result.fold({ value = it }, { failure = it })
        }
    })
    failure?.let { throw it }
    @Suppress("UNCHECKED_CAST")
    return value as T
}
"#,
        )
        .expect("coroutine host-test shim should be written");
        let android_os_support = temp.0.join("AndroidOsSupport.kt");
        fs::write(
            &android_os_support,
            r#"package android.os

class Looper private constructor() {
    companion object {
        private val mainLooper = Looper()
        @JvmStatic fun getMainLooper(): Looper = mainLooper
    }
}

class Handler(@Suppress("UNUSED_PARAMETER") looper: Looper) {
    fun post(operation: Runnable): Boolean {
        operation.run()
        return true
    }
}
"#,
        )
        .expect("Android event main-loop host shim should be written");
        let compiled = Command::new(kotlinc)
            .arg(&coroutine_support)
            .arg(&android_os_support)
            .arg(&package.join("NexaPlugin0_Bindings.kt"))
            .arg(&smoke_test)
            .arg("-include-runtime")
            .arg("-d")
            .arg(&kotlin_jar)
            .output()
            .expect("Kotlin compiler should start");
        assert!(
            compiled.status.success(),
            "generated Kotlin plugin bindings failed to compile:\n{}\n{}",
            String::from_utf8_lossy(&compiled.stdout),
            String::from_utf8_lossy(&compiled.stderr)
        );
    }

    let jni = output.join("android/app/src/main/cpp/Plugin0/NexaPluginJni.cpp");
    assert!(jni.is_file(), "generated JNI source should be staged");
    let cmake = fs::read_to_string(output.join("android/app/src/main/cpp/CMakeLists.txt"))
        .expect("generated CMake target should be present");
    assert!(cmake.contains("Plugin0/NexaPluginJni.cpp"));
    assert!(cmake.contains("add_library(nexa_plugins SHARED"));
    assert!(cmake.contains("set(CMAKE_CXX_STANDARD 17)"));
    let proguard = fs::read_to_string(output.join("android/app/proguard-rules.pro"))
        .expect("generated JNI R8 keep rule should be present");
    assert!(
        proguard.contains("-keep class com.nexa.androidcppjnitest.NexaPlugin0_CppBindings { *; }")
    );
    assert!(
        proguard
            .contains("-keep class com.nexa.androidcppjnitest.NexaPlugin0_CppErrorFactory { *; }")
    );
    assert!(proguard.contains("-keep class com.nexa.androidcppjnitest.LookupError$* { *; }"));
    assert!(proguard.contains(
        "-keep class com.nexa.androidcppjnitest.NexaPlugin0_CppEventMeter_changed { *; }"
    ));

    let native_root = output.join("android/app/src/main/cpp");
    let sdk = env::var_os("ANDROID_HOME")
        .or_else(|| env::var_os("ANDROID_SDK_ROOT"))
        .map(PathBuf::from);
    if let Some(compiler) = sdk.as_deref().and_then(android_ndk_compiler) {
        for source in [jni, native_root.join("Plugin0/cpp/Sources/Plugin.cpp")] {
            let compiled = Command::new(&compiler)
                .args(["-std=c++20", "-fsyntax-only"])
                .arg("-I")
                .arg(native_root.join("Plugin0"))
                .arg(&source)
                .output()
                .expect("Android NDK C++ compiler should start");
            assert!(
                compiled.status.success(),
                "generated Android C++ source failed to compile with {}:\n{}\n{}",
                compiler.display(),
                String::from_utf8_lossy(&compiled.stdout),
                String::from_utf8_lossy(&compiled.stderr)
            );
        }
        let linked = Command::new(&compiler)
            .args(["-std=c++20", "-shared", "-fPIC"])
            .arg("-I")
            .arg(native_root.join("Plugin0"))
            .arg(native_root.join("Plugin0/NexaPluginJni.cpp"))
            .arg(native_root.join("Plugin0/cpp/Sources/Plugin.cpp"))
            .arg("-o")
            .arg(temp.0.join("libnexa_plugins.so"))
            .output()
            .expect("Android NDK linker should start");
        assert!(
            linked.status.success(),
            "generated Android JNI and C++ implementation failed to link:\n{}\n{}",
            String::from_utf8_lossy(&linked.stdout),
            String::from_utf8_lossy(&linked.stderr)
        );
    }

    if cfg!(target_os = "macos")
        && kotlinc.is_some()
        && command_available("clang++", &["--version"])
        && command_available("java", &["-version"])
        && let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from)
    {
        let host_library = Command::new("clang++")
            .args(["-std=c++20", "-dynamiclib", "-fPIC", "-I"])
            .arg(java_home.join("include"))
            .arg("-I")
            .arg(java_home.join("include/darwin"))
            .arg("-I")
            .arg(native_root.join("Plugin0"))
            .arg(native_root.join("Plugin0/NexaPluginJni.cpp"))
            .arg(native_root.join("Plugin0/cpp/Sources/Plugin.cpp"))
            .arg("-o")
            .arg(temp.0.join("libnexa_plugins.dylib"))
            .output()
            .expect("host C++ JNI library should build");
        assert!(
            host_library.status.success(),
            "host JNI smoke library failed to build:\n{}\n{}",
            String::from_utf8_lossy(&host_library.stdout),
            String::from_utf8_lossy(&host_library.stderr)
        );
        let runtime = Command::new("java")
            .arg(format!("-Djava.library.path={}", temp.0.display()))
            .arg("-jar")
            .arg(&kotlin_jar)
            .output()
            .expect("JNI runtime smoke test should start");
        assert!(
            runtime.status.success(),
            "generated Android Kotlin/JNI contract failed at runtime:\n{}\n{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
    }
}

#[test]
fn generated_cpp_native_classes_import_as_owned_swift_references_when_available() {
    if !command_available("swiftc", &["--version"]) {
        return;
    }

    let temp = TempProject::new("cpp-swift-interop");
    fs::create_dir_all(&temp.0).expect("temporary plugin directory should be created");
    fs::write(
        temp.0.join("plugin.config.nx"),
        r#"plugin {
    schema: 2
    id: "dev.example.swift-interop"
    version: "1.0.0"
    sources { native: "native.nxid" }
}

"#,
    )
    .expect("temporary manifest should be written");
    let contract = temp.0.join("native.nxid");
    fs::write(
        &contract,
        r#"native class Counter {
    init()
    readonly property value: Int32
    fn increment()
}
"#,
    )
    .expect("native contract should be written");

    let header = temp.0.join("NexaPluginBindings.hpp");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("plugin")
        .arg("generate")
        .arg(&contract)
        .args(["--target", "cpp", "--out"])
        .arg(&header)
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "C++ contract generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let bridge = temp.0.join("Bridge.h");
    fs::write(&bridge, "#include \"NexaPluginBindings.hpp\"\n")
        .expect("temporary bridging header should be written");
    let swift_source = temp.0.join("Interop.swift");
    fs::write(
        &swift_source,
        "import CxxStdlib\n\nfunc readCppCounter() -> Int32 {\n    let counter = plugin_dev.plugin_example.plugin_swift_dash_interop.nexaMakeCounterForSwift()\n    counter.increment()\n    return counter.getValue()\n}\n",
    )
    .expect("Swift interoperability probe should be written");
    let checked = Command::new("swiftc")
        .args([
            "-typecheck",
            "-cxx-interoperability-mode=default",
            "-import-objc-header",
        ])
        .arg(&bridge)
        .args(["-Xcc", "-std=c++20", "-Xcc"])
        .arg(format!("-I{}", temp.0.display()))
        .arg(&swift_source)
        .output()
        .expect("swiftc should start when available");
    assert!(
        checked.status.success(),
        "generated C++ reference contract failed Swift C++ interoperability type-checking:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn generated_ios_cpp_adapters_typecheck_primitive_nullable_and_collection_matrix_when_swift_is_available()
 {
    if !command_available("swiftc", &["--version"]) {
        return;
    }

    let temp = TempProject::new("ios-cpp-type-matrix");
    let (entry, _) = type_matrix_plugin(&temp, false);
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "ios", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("CppTypeMatrix")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "iOS type-matrix project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let ios = output.join("ios/CppTypeMatrix");
    let generated_cpp = ios.join("NexaPluginCpp/Plugin0/cpp/Sources/Plugin.cpp");
    let checked_cpp = Command::new("clang++")
        .args(["-std=c++20", "-fsyntax-only", "-I"])
        .arg(ios.join("NexaPluginCpp/Plugin0"))
        .arg(&generated_cpp)
        .output()
        .expect("clang++ should start when available");
    assert!(
        checked_cpp.status.success(),
        "iOS generated C++ type matrix failed to compile:\n{}",
        String::from_utf8_lossy(&checked_cpp.stderr)
    );

    let bindings = ios.join("NexaPlugins/NexaPlugin0_Bindings.swift");
    let adapters = ios.join("NexaPlugins/NexaPlugin0_CppBindings.swift");
    assert!(
        bindings.is_file(),
        "generated iOS IDL contract should be staged"
    );
    assert!(
        adapters.is_file(),
        "generated iOS C++ adapters should be staged"
    );
    let probe = temp.0.join("TypeMatrixProbe.swift");
    fs::write(&probe, type_matrix_swift_probe())
        .expect("Swift type-matrix probe should be written");
    let bridge = ios.join("NexaPluginCpp-Bridging-Header.h");
    let checked = Command::new("swiftc")
        .args([
            "-typecheck",
            "-cxx-interoperability-mode=default",
            "-import-objc-header",
        ])
        .arg(&bridge)
        .args(["-Xcc", "-std=c++20", "-Xcc"])
        .arg(format!("-I{}", ios.display()))
        .arg(&bindings)
        .arg(&adapters)
        .arg(&probe)
        .output()
        .expect("swiftc should start when available");
    assert!(
        checked.status.success(),
        "generated iOS Swift/C++ adapters failed to type-check the primitive and collection matrix:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn generated_android_cpp_adapters_roundtrip_primitive_nullable_and_collection_matrix_when_toolchains_are_available()
 {
    let temp = TempProject::new("android-cpp-type-matrix");
    let (entry, _) = type_matrix_plugin(&temp, true);
    let output = temp.0.join("Generated");
    let generated = Command::new(env!("CARGO_BIN_EXE_nexa"))
        .arg("generate")
        .arg(&entry)
        .args(["--target", "android", "--out"])
        .arg(&output)
        .arg("--name")
        .arg("CppTypeMatrixAndroid")
        .output()
        .expect("Nexa CLI should start");
    assert!(
        generated.status.success(),
        "Android type-matrix project generation failed:\n{}\n{}",
        String::from_utf8_lossy(&generated.stdout),
        String::from_utf8_lossy(&generated.stderr)
    );

    let package = output.join("android/app/src/main/java/com/nexa/cpptypematrixandroid");
    let bindings = package.join("NexaPlugin0_Bindings.kt");
    let generated_kotlin =
        fs::read_to_string(&bindings).expect("generated Android C++ bindings should be staged");
    let gradle = fs::read_to_string(output.join("android/app/build.gradle.kts"))
        .expect("generated Android Gradle configuration should be readable");
    assert!(
        !gradle.contains("kotlinx-coroutines-android"),
        "synchronous C++ plugins should not pull in coroutine dependencies"
    );
    for ty in CXX_PRIMITIVE_TYPES {
        assert!(
            generated_kotlin.contains(&format!("echo{}(value: {})", ty.suffix, ty.kotlin)),
            "generated Android binding is missing {} scalar mapping",
            ty.idl
        );
        assert!(
            generated_kotlin.contains(&format!(
                "maybe{}(value: {}?): {}?",
                ty.suffix, ty.kotlin, ty.kotlin
            )),
            "generated Android binding is missing nullable {} mapping",
            ty.idl
        );
        assert!(
            generated_kotlin.contains(&format!(
                "echoArray{}(values: List<{}>): List<{}>",
                ty.suffix, ty.kotlin, ty.kotlin
            )),
            "generated Android binding is missing Array<{}> mapping",
            ty.idl
        );
        assert!(generated_kotlin.contains(&format!(
            "echoOptionalArray{}(values: List<{}?>): List<{}?>",
            ty.suffix, ty.kotlin, ty.kotlin
        )));
    }

    assert!(generated_kotlin.contains(
        "external fun service_Types_echoArrayMap(values: Map<Int, IntArray>): Map<Int, IntArray>"
    ));
    assert!(
        generated_kotlin.contains(
            "override fun echoArrayMap(values: Map<Int, List<Int>>): Map<Int, List<Int>>"
        )
    );
    assert!(generated_kotlin.contains("nexaMapValue.toIntArray()"));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoBytesMap(values: Map<Int, ByteArray>): Map<Int, ByteArray>"
    ));
    assert!(
        generated_kotlin.contains(
            "override fun echoBytesMap(values: Map<Int, ByteArray>): Map<Int, ByteArray>"
        )
    );
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoNestedMap(values: Map<Int, Map<String, ByteArray>>): Map<Int, Map<String, ByteArray>>"
    ));
    assert!(generated_kotlin.contains(
        "override fun echoNestedMap(values: Map<Int, Map<String, ByteArray>>): Map<Int, Map<String, ByteArray>>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoOptionalNestedMap(values: Map<Int, Map<String, ByteArray>?>): Map<Int, Map<String, ByteArray>?>"
    ));
    assert!(generated_kotlin.contains(
        "override fun echoOptionalNestedMap(values: Map<Int, Map<String, ByteArray>?>): Map<Int, Map<String, ByteArray>?>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoOptionalMap(values: Map<Int, ByteArray>?): Map<Int, ByteArray>?"
    ));
    assert!(generated_kotlin.contains(
        "override fun echoOptionalMap(values: Map<UInt, ByteArray>?): Map<UInt, ByteArray>?"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoNullableValueMap(values: Map<Int, Int?>): Map<Int, Int?>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoOptionalSet(values: Array<Long?>): Array<Long?>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoNullableArrayMap(values: Map<Int, Array<Int?>>): Map<Int, Array<Int?>>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoNullableSetMap(values: Map<Int, Array<Long?>>): Map<Int, Array<Long?>>"
    ));
    assert!(
        generated_kotlin.contains(
            "values?.let { nexaOptionalMap -> nexaOptionalMap.mapKeys { it.key.toInt() } }"
        )
    );
    assert!(generated_kotlin.contains("nexaOptionalMap.mapKeys { it.key.toUInt() }"));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoStringArrayMap(values: Map<Int, Array<String>>): Map<Int, Array<String>>"
    ));
    assert!(generated_kotlin.contains(
        "override fun echoStringArrayMap(values: Map<Int, List<String>>): Map<Int, List<String>>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoBytesArrayMap(values: Map<Int, Array<ByteArray>>): Map<Int, Array<ByteArray>>"
    ));
    assert!(generated_kotlin.contains(
        "override fun echoBytesArrayMap(values: Map<Int, List<ByteArray>>): Map<Int, List<ByteArray>>"
    ));
    assert!(generated_kotlin.contains(
        "external fun service_Types_echoSetMap(values: Map<Int, IntArray>): Map<Int, IntArray>"
    ));
    assert!(
        generated_kotlin
            .contains("override fun echoSetMap(values: Map<Int, Set<UInt>>): Map<Int, Set<UInt>>")
    );

    if command_available("clang++", &["--version"])
        && let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from)
    {
        let native_root = output.join("android/app/src/main/cpp");
        let platform_include = if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "windows") {
            "win32"
        } else {
            "linux"
        };
        let checked = Command::new("clang++")
            .args(["-std=c++20", "-fsyntax-only", "-I"])
            .arg(java_home.join("include"))
            .arg("-I")
            .arg(java_home.join("include").join(platform_include))
            .arg("-I")
            .arg(native_root.join("Plugin0"))
            .arg(native_root.join("Plugin0/NexaPluginJni.cpp"))
            .arg(native_root.join("Plugin0/cpp/Sources/Plugin.cpp"))
            .output()
            .expect("host C++ compiler should start when available");
        assert!(
            checked.status.success(),
            "generated Android JNI map-of-arrays adapters failed to compile:\n{}\n{}",
            String::from_utf8_lossy(&checked.stdout),
            String::from_utf8_lossy(&checked.stderr)
        );
    }

    let smoke = package.join("TypeMatrixSmoke.kt");
    fs::write(&smoke, type_matrix_kotlin_smoke())
        .expect("Kotlin type-matrix runtime test should be written");
    let Some(kotlinc) = kotlin_compiler() else {
        return;
    };
    let kotlin_jar = temp.0.join("TypeMatrixSmoke.jar");
    let compiled = Command::new(&kotlinc)
        .arg(&bindings)
        .arg(&smoke)
        .arg("-include-runtime")
        .arg("-d")
        .arg(&kotlin_jar)
        .output()
        .expect("Kotlin compiler should start when available");
    assert!(
        compiled.status.success(),
        "generated Android C++ bindings failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );

    #[cfg(target_os = "macos")]
    if command_available("clang++", &["--version"])
        && command_available("java", &["-version"])
        && let Some(java_home) = env::var_os("JAVA_HOME").map(PathBuf::from)
    {
        let native_root = output.join("android/app/src/main/cpp");
        let jni = native_root.join("Plugin0/NexaPluginJni.cpp");
        let cpp_source = native_root.join("Plugin0/cpp/Sources/Plugin.cpp");
        let library = temp.0.join("libnexa_plugins.dylib");
        let linked = Command::new("clang++")
            .args(["-std=c++20", "-dynamiclib", "-fPIC", "-I"])
            .arg(java_home.join("include"))
            .arg("-I")
            .arg(java_home.join("include/darwin"))
            .arg("-I")
            .arg(native_root.join("Plugin0"))
            .arg(&jni)
            .arg(&cpp_source)
            .arg("-o")
            .arg(&library)
            .output()
            .expect("host C++ JNI linker should start");
        assert!(
            linked.status.success(),
            "generated Android C++/JNI primitive matrix failed to link:\n{}\n{}",
            String::from_utf8_lossy(&linked.stdout),
            String::from_utf8_lossy(&linked.stderr)
        );
        let runtime = Command::new("java")
            .arg(format!("-Djava.library.path={}", temp.0.display()))
            .arg("-jar")
            .arg(&kotlin_jar)
            .output()
            .expect("headless Kotlin/JNI type-matrix test should start");
        assert!(
            runtime.status.success(),
            "generated Android Kotlin/JNI primitive round trips failed:\n{}\n{}",
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );
    }
}
