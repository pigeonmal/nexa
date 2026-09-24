use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use nexa_syntax::ast::PluginDependencyConfig;

#[derive(Debug)]
pub(super) struct ResolvedDependencies {
    pub(super) plugin_roots: HashMap<String, PathBuf>,
    pub(super) lock_file: String,
}

pub(super) fn resolve(
    project_root: &Path,
    dependencies: &[PluginDependencyConfig],
) -> Result<ResolvedDependencies, String> {
    let mut roots = HashMap::new();
    let mut lock_entries = BTreeMap::new();
    for dependency in dependencies {
        let (root, source, revision, package) = if let Some(path) = &dependency.path {
            let root = project_root.join(path);
            (root, format!("path:{path}"), None, None)
        } else {
            let git = dependency.git.as_deref().ok_or_else(|| {
                format!("dependency `{}` requires `path` or `git`", dependency.alias)
            })?;
            let revision = dependency
                .revision
                .as_deref()
                .ok_or_else(|| format!("Git dependency `{}` must pin `rev`", dependency.alias))?;
            if !valid_commit(revision) {
                return Err(format!(
                    "Git dependency `{}` rev must be a full 40- or 64-digit commit hash",
                    dependency.alias
                ));
            }
            let repo = checkout(project_root, git, revision)?;
            let package = dependency.package_path.clone();
            if package
                .as_deref()
                .is_some_and(|path| !valid_package_path(path))
            {
                return Err(format!(
                    "Git dependency `{}` package must be a relative path without `..` segments",
                    dependency.alias
                ));
            }
            let root = package
                .as_deref()
                .map(|subdirectory| repo.join(subdirectory))
                .unwrap_or_else(|| repo.clone());
            (
                root,
                format!("git:{git}"),
                Some(revision.to_ascii_lowercase()),
                package,
            )
        };
        let root = fs::canonicalize(&root).map_err(|error| {
            format!(
                "plugin dependency `{}` at {}: {error}",
                dependency.alias,
                root.display()
            )
        })?;
        let manifest_path = root.join("plugin.config.nx");
        let manifest = nexa_plugin_idl::manifest::parse_file(&manifest_path)
            .map_err(|error| format!("plugin dependency `{}`: {error}", dependency.alias))?;
        if manifest.id != dependency.package_id {
            return Err(format!(
                "plugin dependency `{}` declares id `{}` but package manifest is `{}`",
                dependency.alias, dependency.package_id, manifest.id
            ));
        }
        if let Some(existing) = roots.get(&dependency.package_id)
            && existing != &root
        {
            return Err(format!(
                "plugin package `{}` resolves to more than one root",
                dependency.package_id
            ));
        }
        roots.insert(dependency.package_id.clone(), root);
        lock_entries.insert(
            dependency.alias.clone(),
            LockEntry {
                id: dependency.package_id.clone(),
                source,
                revision,
                package,
            },
        );
    }

    Ok(ResolvedDependencies {
        plugin_roots: roots,
        lock_file: render_lock(&lock_entries),
    })
}

pub(super) fn write_lock(project_root: &Path, contents: &str) -> Result<(), String> {
    let path = project_root.join("nexa.lock");
    if fs::read_to_string(&path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    fs::write(&path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn checkout(project_root: &Path, url: &str, revision: &str) -> Result<PathBuf, String> {
    let git_root = project_root.join(".nexa/plugins").join(format!(
        "{:016x}-{}",
        stable_hash(url.as_bytes()),
        revision.to_ascii_lowercase()
    ));
    if !git_root.join(".git").is_dir() {
        if git_root.exists() {
            fs::remove_dir_all(&git_root)
                .map_err(|error| format!("{}: {error}", git_root.display()))?;
        }
        if let Some(parent) = git_root.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        let output = Command::new("git")
            .args([
                "clone",
                "--quiet",
                "--no-checkout",
                "--filter=blob:none",
                "--",
            ])
            .arg(url)
            .arg(&git_root)
            .output()
            .map_err(|error| format!("cannot start git to fetch `{url}`: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "cannot clone plugin repository `{url}`: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }

    let output = Command::new("git")
        .arg("-C")
        .arg(&git_root)
        .args(["checkout", "--quiet", "--detach"])
        .arg(revision)
        .output()
        .map_err(|error| format!("cannot start git to check out `{revision}`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot check out plugin revision `{revision}` from `{url}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let actual = git_output(&git_root, &["rev-parse", "HEAD"])?;
    if actual != revision.to_ascii_lowercase() {
        return Err(format!(
            "plugin repository `{url}` resolved `{revision}` to unexpected commit `{actual}`"
        ));
    }
    let actual_url = git_output(&git_root, &["config", "--get", "remote.origin.url"])?;
    if actual_url != url {
        return Err(format!(
            "cached plugin checkout {} points to `{actual_url}`, expected `{url}`",
            git_root.display()
        ));
    }
    let changes = git_output(
        &git_root,
        &["status", "--porcelain", "--untracked-files=all"],
    )?;
    if !changes.is_empty() {
        return Err(format!(
            "cached plugin checkout {} has local changes; remove it to restore the pinned source",
            git_root.display()
        ));
    }
    Ok(git_root)
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("cannot start git in {}: {error}", root.display()))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed in {}: {}",
            args.join(" "),
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn valid_commit(revision: &str) -> bool {
    matches!(revision.len(), 40 | 64) && revision.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_package_path(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

struct LockEntry {
    id: String,
    source: String,
    revision: Option<String>,
    package: Option<String>,
}

fn render_lock(entries: &BTreeMap<String, LockEntry>) -> String {
    let plugins = entries
        .iter()
        .map(|(alias, entry)| {
            format!(
                "    {{\"alias\":{},\"id\":{},\"source\":{},\"revision\":{},\"package\":{}}}",
                json_string(alias),
                json_string(&entry.id),
                json_string(&entry.source),
                entry
                    .revision
                    .as_deref()
                    .map(json_string)
                    .unwrap_or_else(|| "null".to_owned()),
                entry
                    .package
                    .as_deref()
                    .map(json_string)
                    .unwrap_or_else(|| "null".to_owned()),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!("{{\n  \"version\": 1,\n  \"plugins\": [\n{plugins}\n  ]\n}}\n")
}

fn json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
