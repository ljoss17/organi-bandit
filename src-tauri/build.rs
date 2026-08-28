fn main() {
    // Tauri resource paths can't escape the src-tauri directory reliably
    // across bundle targets, so stage a fresh copy of the repo-root
    // CHANGELOG.md (the one towncrier writes to) here on every build
    // instead of referencing it in place.
    println!("cargo:rerun-if-changed=../CHANGELOG.md");
    std::fs::copy("../CHANGELOG.md", "resources/CHANGELOG.md")
        .expect("failed to copy CHANGELOG.md into src-tauri/resources for bundling");

    tauri_build::build()
}
