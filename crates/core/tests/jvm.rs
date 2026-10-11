//! JVM-Presets beim Start: globaler Standard, Wahl der Instanz, Java-Mindestversion.

use pinecone_core::download::Downloader;
use pinecone_core::jvm::{NONE, StoredPreset};
use pinecone_core::launcher::Launcher;
use pinecone_core::paths::Paths;
use pinecone_core::profiles::Profile;

#[test]
fn launch_uses_default_or_instance_preset() {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::new(
        Paths::new(dir.path().join("data")),
        Downloader::new().unwrap(),
    )
    .unwrap();
    let mut profile = Profile::new("Test", "1.21.1");
    profile.jvm_args = vec!["-Dmein.flag=1".into()];

    // Ohne Preset: nur die eigenen Argumente.
    assert_eq!(launcher.jvm_args(&profile, Some(21)), vec!["-Dmein.flag=1"]);

    // Globaler Standard: Aikar's Flags, eigene Argumente zuletzt.
    let mut settings = launcher.settings().load().unwrap();
    settings.jvm.default_preset = "aikar".into();
    settings.jvm.presets.push(StoredPreset {
        id: "leicht".into(),
        name: "Leicht".into(),
        args: vec!["-XX:+UseSerialGC".into()],
        min_java: 8,
    });
    launcher.settings().save(&settings).unwrap();
    let args = launcher.jvm_args(&profile, Some(21));
    assert_eq!(args[0], "-XX:+UseG1GC");
    assert_eq!(args.last().unwrap(), "-Dmein.flag=1");

    // Instanz wählt ein eigenes Preset bzw. keines.
    profile.jvm_preset = Some("leicht".into());
    assert_eq!(
        launcher.jvm_args(&profile, Some(8)),
        vec!["-XX:+UseSerialGC", "-Dmein.flag=1"]
    );
    profile.jvm_preset = Some(NONE.into());
    assert_eq!(launcher.jvm_args(&profile, Some(21)), vec!["-Dmein.flag=1"]);

    // ZGC auf Java 17: Preset entfällt, Spiel startet trotzdem.
    profile.jvm_preset = Some("zgc".into());
    assert_eq!(launcher.jvm_args(&profile, Some(17)), vec!["-Dmein.flag=1"]);
    assert!(
        launcher
            .jvm_args(&profile, Some(21))
            .contains(&"-XX:+UseZGC".to_string())
    );

    // Ungültige Presets werden nicht gespeichert.
    settings.jvm.presets.push(StoredPreset {
        id: "kaputt".into(),
        name: "Kaputt".into(),
        args: vec!["-Xmx64G".into()],
        min_java: 8,
    });
    assert!(launcher.settings().save(&settings).is_err());
}
