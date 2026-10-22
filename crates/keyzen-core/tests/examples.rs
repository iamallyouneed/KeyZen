use keyzen_core::Config;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

#[test]
fn bundled_examples_are_valid() {
    for name in ["keyzen.yaml", "fullkeyzen.yaml"] {
        let path = examples_dir().join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        Config::from_yaml_str(&text)
            .unwrap_or_else(|error| panic!("invalid example {}: {error}", path.display()));
    }
}
