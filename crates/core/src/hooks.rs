//! Start-Befehle einer Instanz (F-55): Befehl vor dem Start, Wrapper um Java, Befehl nach dem
//! Ende. Sie kommen nur aus der Eingabe des Nutzers, nie aus Packs oder Manifesten, und laufen
//! ohne Shell: Der Text wird wie in einer Shell in Wörter zerlegt (Anführungszeichen, `\`),
//! Platzhalter werden je Wort ersetzt.

use std::path::Path;
use std::process::Stdio;

use crate::{Error, Result};

/// Werte für die Platzhalter `$INST_DIR`, `$INST_NAME`, `$INST_ID` und `$MC_VERSION`.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub inst_dir: String,
    pub inst_name: String,
    pub inst_id: String,
    pub mc_version: String,
}

impl HookContext {
    fn vars(&self) -> [(&'static str, &str); 4] {
        [
            ("INST_DIR", &self.inst_dir),
            ("INST_NAME", &self.inst_name),
            ("INST_ID", &self.inst_id),
            ("MC_VERSION", &self.mc_version),
        ]
    }

    fn expand(&self, word: &str) -> String {
        let mut out = word.to_string();
        for (name, value) in self.vars() {
            out = out
                .replace(&format!("${{{name}}}"), value)
                .replace(&format!("${name}"), value);
        }
        out
    }
}

/// Zerlegt einen Befehl in Wörter: Leerzeichen trennen, `"…"` und `'…'` fassen zusammen,
/// `\` maskiert das nächste Zeichen (außer in `'…'`). Unter Windows bleibt `\` in Pfaden stehen,
/// wenn danach kein Leerzeichen oder Anführungszeichen folgt.
pub fn split(command: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => current.push(c),
                        None => return Err(Error::Invalid("Fehlendes ' im Befehl".into())),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') if matches!(chars.peek(), Some('"' | '\\')) => {
                            current.push(chars.next().unwrap_or('\\'));
                        }
                        Some(c) => current.push(c),
                        None => return Err(Error::Invalid("Fehlendes \" im Befehl".into())),
                    }
                }
            }
            '\\' => {
                in_word = true;
                match chars.peek() {
                    Some(&n) if n.is_whitespace() || matches!(n, '"' | '\'' | '\\') => {
                        current.push(n);
                        chars.next();
                    }
                    _ => current.push('\\'),
                }
            }
            c if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut current));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                current.push(c);
            }
        }
    }
    if in_word {
        words.push(current);
    }
    Ok(words)
}

/// Zerlegt und ersetzt Platzhalter; leerer Befehl → `None`.
pub fn prepare(command: &str, ctx: &HookContext) -> Result<Option<Vec<String>>> {
    let words = split(command)?;
    if words.is_empty() {
        return Ok(None);
    }
    Ok(Some(words.iter().map(|w| ctx.expand(w)).collect()))
}

/// Ergebnis eines Befehls: Exit-Code und die Ausgabe (stdout und stderr) zeilenweise.
#[derive(Debug, Clone)]
pub struct HookOutput {
    pub success: bool,
    pub code: Option<i32>,
    pub lines: Vec<String>,
}

/// Führt einen vorbereiteten Befehl im Spielordner aus; die Platzhalter stehen zusätzlich als
/// Umgebungsvariablen bereit.
pub async fn run(words: &[String], cwd: &Path, ctx: &HookContext) -> Result<HookOutput> {
    let (program, args) = words
        .split_first()
        .ok_or_else(|| Error::Invalid("Leerer Befehl".into()))?;
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (name, value) in ctx.vars() {
        cmd.env(name, value);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| Error::Invalid(format!("Befehl {program:?} nicht ausführbar: {e}")))?;
    let mut lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    lines.extend(
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .map(str::to_string),
    );
    Ok(HookOutput {
        success: output.status.success(),
        code: output.status.code(),
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> HookContext {
        HookContext {
            inst_dir: "/data/instances/avalon/.minecraft".into(),
            inst_name: "REVIVAL: Avalon".into(),
            inst_id: "avalon".into(),
            mc_version: "1.21.11".into(),
        }
    }

    #[test]
    fn splits_like_a_shell() {
        assert_eq!(split("gamemoderun").unwrap(), ["gamemoderun"]);
        assert_eq!(
            split(r#"  prime-run   env  "A B"  'c "d"' e\ f  "x\"y" "" "#).unwrap(),
            ["prime-run", "env", "A B", r#"c "d""#, "e f", r#"x"y"#, ""]
        );
        // Windows-Pfade behalten ihre Backslashes.
        assert_eq!(
            split(r"C:\Tools\wrap.exe --x").unwrap(),
            [r"C:\Tools\wrap.exe", "--x"]
        );
        assert!(split(r#"echo "offen"#).is_err());
        assert!(split("echo 'offen").is_err());
        assert!(split("   ").unwrap().is_empty());
    }

    #[test]
    fn expands_placeholders_per_word() {
        let words = prepare(
            r#"backup.sh "$INST_DIR/saves" ${INST_NAME} $MC_VERSION"#,
            &ctx(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            words,
            [
                "backup.sh",
                "/data/instances/avalon/.minecraft/saves",
                "REVIVAL: Avalon",
                "1.21.11"
            ]
        );
        assert!(prepare("", &ctx()).unwrap().is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn runs_without_shell_and_reports_failure() {
        let dir = tempfile::tempdir().unwrap();
        let ok = run(
            &[
                "sh".into(),
                "-c".into(),
                "echo $INST_ID; echo fehler >&2".into(),
            ],
            dir.path(),
            &ctx(),
        )
        .await
        .unwrap();
        assert!(ok.success);
        assert_eq!(ok.lines, ["avalon", "fehler"]);

        let fail = run(
            &["sh".into(), "-c".into(), "exit 3".into()],
            dir.path(),
            &ctx(),
        )
        .await
        .unwrap();
        assert!(!fail.success);
        assert_eq!(fail.code, Some(3));

        // Ein Wort mit `;` ist ein Argument, keine zweite Anweisung.
        let literal = run(&["echo".into(), "a; rm -rf x".into()], dir.path(), &ctx())
            .await
            .unwrap();
        assert_eq!(literal.lines, ["a; rm -rf x"]);
        assert!(
            run(&["gibt-es-nicht-xyz".into()], dir.path(), &ctx())
                .await
                .is_err()
        );
    }
}
