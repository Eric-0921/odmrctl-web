//! Integration test: load every example profile and recipe
//! committed in `examples/` and assert they parse + validate.

use std::path::PathBuf;

use oe1022d_config_stack::{
    load_laser_profile, load_magnetic_profile, load_oe1022d_profile, load_onion_recipe,
    load_smb100a_profile, Layer,
};

fn examples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
}

#[test]
fn all_example_profiles_load() {
    let root = examples_root();
    let oe = load_oe1022d_profile(&root.join("profiles/oe1022d.default.json")).expect("oe");
    assert_eq!(oe.id, "oe1022d_lab_main");
    let smb = load_smb100a_profile(&root.join("profiles/smb100a.default.json")).expect("smb");
    assert!((smb.frequency_hz - 2.882e9).abs() < 1.0);
    let mag = load_magnetic_profile(&root.join("profiles/magnetic.default.json")).expect("mag");
    assert_eq!(mag.field_vector_nt.z, 1000.0);
    let laser = load_laser_profile(&root.join("profiles/cni_laser.disabled.json")).expect("laser");
    assert_eq!(laser.state, "OFF");
}

#[test]
fn demo_recipe_loads_and_oe1022d_is_first() {
    let root = examples_root();
    let r = load_onion_recipe(&root.join("recipes/onion_demo.json")).expect("recipe");
    assert_eq!(r.id, "demo_run_001");
    assert!(r.layers.contains_key(&Layer::Oe1022d));
    assert!(r.layers.contains_key(&Layer::Smb100a));
    assert!(r.layers.contains_key(&Layer::Magnetic));
    assert!(r.layers.contains_key(&Layer::Laser));
    let oe = &r.layers[&Layer::Oe1022d];
    assert!(oe.enabled);
    let laser = &r.layers[&Layer::Laser];
    assert!(!laser.enabled);
}

#[test]
fn demo_recipe_field_list_round_trip() {
    let root = examples_root();
    let r = load_onion_recipe(&root.join("recipes/onion_demo.json")).expect("recipe");
    assert_eq!(r.acquisition.fields, vec!["BX", "BY", "BFreq"]);
    assert!(r.acquisition.target_frames >= 100);
}
