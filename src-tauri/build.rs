fn main() {
    // Tauri resource paths can't escape the src-tauri directory reliably
    // across bundle targets, so stage a fresh copy of the repo-root
    // CHANGELOG.md (the one towncrier writes to) here on every build
    // instead of referencing it in place.
    println!("cargo:rerun-if-changed=../CHANGELOG.md");

    let changelog = std::fs::read("../CHANGELOG.md").expect("failed to read ../CHANGELOG.md");
    // Only write when the content actually changed: `cargo tauri dev`'s file
    // watcher covers src-tauri/resources, so an unconditional write here
    // would touch the destination's mtime on every build, which the watcher
    // sees as a source change and uses to trigger another rebuild, looping
    // forever.
    if std::fs::read("resources/CHANGELOG.md").ok().as_deref() != Some(changelog.as_slice()) {
        std::fs::write("resources/CHANGELOG.md", &changelog)
            .expect("failed to copy CHANGELOG.md into src-tauri/resources for bundling");
    }

    tauri_build::build()
}
