//! Screenshots der Profile (`<spielordner>/screenshots/*.png`).

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub profile_id: String,
    pub path: PathBuf,
    /// Unix-Zeit (Sekunden) der letzten Änderung.
    pub modified: u64,
}

/// Screenshots eines Spielordners.
pub fn list(profile_id: &str, game_dir: &Path) -> Vec<Screenshot> {
    let Ok(entries) = std::fs::read_dir(game_dir.join("screenshots")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("png"))
        })
        .filter_map(|e| {
            let modified = e
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some(Screenshot {
                profile_id: profile_id.to_string(),
                path: e.path(),
                modified,
            })
        })
        .collect()
}

/// Die neuesten `limit` Screenshots über alle Spielordner.
pub fn latest<'a>(
    dirs: impl IntoIterator<Item = (&'a str, PathBuf)>,
    limit: usize,
) -> Vec<Screenshot> {
    let mut all: Vec<Screenshot> = dirs
        .into_iter()
        .flat_map(|(id, dir)| list(id, &dir))
        .collect();
    all.sort_by_key(|s| std::cmp::Reverse(s.modified));
    all.truncate(limit);
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_latest_pngs() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        for (d, name, age) in [
            (&a, "alt.png", 300),
            (&a, "neu.png", 10),
            (&b, "mittel.PNG", 100),
            (&b, "notiz.txt", 0),
        ] {
            std::fs::create_dir_all(d.join("screenshots")).unwrap();
            let f = d.join("screenshots").join(name);
            std::fs::write(&f, b"x").unwrap();
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(age);
            std::fs::File::options()
                .write(true)
                .open(&f)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        let shots = latest(
            [
                ("a", a.clone()),
                ("b", b.clone()),
                ("c", dir.path().join("fehlt")),
            ],
            2,
        );
        let names: Vec<String> = shots
            .iter()
            .map(|s| s.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["neu.png", "mittel.PNG"]);
        assert_eq!(shots[1].profile_id, "b");
    }
}
