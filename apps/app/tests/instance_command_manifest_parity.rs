use std::collections::BTreeSet;

fn bracket_contents_after<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing source marker: {marker}"))
        + marker.len();
    let mut depth = 1_u32;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return &source[start..start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated command list after marker: {marker}");
}

fn runtime_commands(source: &str) -> BTreeSet<&str> {
    bracket_contents_after(source, "tauri::generate_handler![")
        .split(',')
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .collect()
}

fn manifest_commands(source: &str) -> BTreeSet<&str> {
    let instance_plugin = source
        .find(".plugin(\n                \"instance\",")
        .expect("missing instance InlinedPlugin");
    bracket_contents_after(&source[instance_plugin..], ".commands(&[")
        .split(',')
        .map(str::trim)
        .filter_map(|command| command.strip_prefix('"')?.strip_suffix('"'))
        .collect()
}

#[test]
fn instance_runtime_handlers_are_registered_in_build_manifest() {
    let runtime = runtime_commands(include_str!("../src/api/instance.rs"));
    let manifest = manifest_commands(include_str!("../build.rs"));
    let missing = runtime.difference(&manifest).copied().collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "instance commands missing from build.rs manifest: {}",
        missing.join(", ")
    );
    for command in [
        "instance_get_post_upgrade_notice",
        "instance_dismiss_post_upgrade_notice",
    ] {
        assert!(
            runtime.contains(command),
            "runtime handler missing {command}"
        );
        assert!(
            manifest.contains(command),
            "build.rs manifest missing {command}"
        );
    }
}
