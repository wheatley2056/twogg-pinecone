//! Paralleler Downloader mit SHA-1-Prüfung, Retry und Wiederverwendung vorhandener Dateien.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::{StreamExt, TryStreamExt};
use sha1::{Digest, Sha1};
use tokio::io::AsyncWriteExt;

use crate::{Error, Result};

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Started {
        files: usize,
        bytes: u64,
    },
    FileDone {
        done: usize,
        files: usize,
        bytes_done: u64,
        cached: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Downloader {
    client: reqwest::Client,
    concurrency: usize,
    retries: u32,
    retry_delay: Duration,
    verify_existing: bool,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(crate::user_agent())
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()
            .map_err(|source| Error::Http {
                url: String::new(),
                source,
            })?;
        Ok(Self {
            client,
            concurrency: 16,
            retries: 3,
            retry_delay: Duration::from_millis(500),
            verify_existing: false,
        })
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n.max(1);
        self
    }

    pub fn retries(mut self, retries: u32, delay: Duration) -> Self {
        self.retries = retries;
        self.retry_delay = delay;
        self
    }

    /// Vorhandene Dateien per SHA-1 statt nur per Größe prüfen (langsamer, aber gründlich).
    pub fn verify_existing(mut self, verify: bool) -> Self {
        self.verify_existing = verify;
        self
    }

    /// Lädt eine kleine Datei (Manifest, Loader-Profil) in den Speicher. Netzwerkfehler und
    /// Serverfehler (5xx) werden wiederholt, Client-Fehler (4xx) sofort gemeldet.
    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut attempt = 0;
        loop {
            match self.try_fetch(url).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) if attempt < self.retries && !is_client_error(&e) => {
                    attempt += 1;
                    tracing::warn!("Abruf fehlgeschlagen (Versuch {attempt}): {e}");
                    tokio::time::sleep(self.retry_delay * 2u32.pow(attempt - 1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn try_fetch(&self, url: &str) -> Result<Vec<u8>> {
        let http = |source| Error::Http {
            url: url.to_string(),
            source,
        };
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http)?;
        Ok(resp.bytes().await.map_err(http)?.to_vec())
    }

    /// Lädt eine Datei, falls sie fehlt oder nicht passt. Liefert `true`, wenn geladen wurde.
    pub async fn download(&self, task: &DownloadTask) -> Result<bool> {
        if self.is_valid(task).await? {
            return Ok(false);
        }
        let mut attempt = 0;
        loop {
            match self.try_download(task).await {
                Ok(()) => return Ok(true),
                Err(e) if attempt < self.retries => {
                    attempt += 1;
                    tracing::warn!("Download fehlgeschlagen (Versuch {attempt}): {e}");
                    tokio::time::sleep(self.retry_delay * 2u32.pow(attempt - 1)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Lädt alle Dateien parallel. Doppelte Ziele werden nur einmal geladen.
    pub async fn download_all(
        &self,
        tasks: Vec<DownloadTask>,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<()> {
        let mut seen = HashSet::new();
        let tasks: Vec<DownloadTask> = tasks
            .into_iter()
            .filter(|t| seen.insert(t.dest.clone()))
            .collect();
        let files = tasks.len();
        let bytes = tasks.iter().filter_map(|t| t.size).sum();
        on_progress(Progress::Started { files, bytes });

        let mut results = futures::stream::iter(tasks)
            .map(|task| async move {
                let downloaded = self.download(&task).await?;
                Ok::<_, Error>((task.size.unwrap_or(0), downloaded))
            })
            .buffer_unordered(self.concurrency);

        let (mut done, mut bytes_done) = (0, 0);
        while let Some((size, downloaded)) = results.try_next().await? {
            done += 1;
            bytes_done += size;
            on_progress(Progress::FileDone {
                done,
                files,
                bytes_done,
                cached: !downloaded,
            });
        }
        Ok(())
    }

    async fn is_valid(&self, task: &DownloadTask) -> Result<bool> {
        let meta = match tokio::fs::metadata(&task.dest).await {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(Error::io(&task.dest)(e)),
        };
        if let Some(size) = task.size {
            if meta.len() != size {
                return Ok(false);
            }
            if !self.verify_existing {
                return Ok(true);
            }
        }
        match &task.sha1 {
            Some(expected) => Ok(sha1_file(&task.dest).await?.eq_ignore_ascii_case(expected)),
            None => Ok(true),
        }
    }

    async fn try_download(&self, task: &DownloadTask) -> Result<()> {
        let http = |source| Error::Http {
            url: task.url.clone(),
            source,
        };
        if let Some(parent) = task.dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(Error::io(parent))?;
        }
        let part = part_path(&task.dest);
        let result = async {
            let resp = self
                .client
                .get(&task.url)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(http)?;
            let mut file = tokio::fs::File::create(&part)
                .await
                .map_err(Error::io(&part))?;
            let mut hasher = Sha1::new();
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(http)?;
                hasher.update(&chunk);
                file.write_all(&chunk).await.map_err(Error::io(&part))?;
            }
            file.flush().await.map_err(Error::io(&part))?;
            drop(file);

            if let Some(expected) = &task.sha1 {
                let actual = format!("{:x}", hasher.finalize());
                if !actual.eq_ignore_ascii_case(expected) {
                    return Err(Error::HashMismatch {
                        path: task.dest.clone(),
                        expected: expected.clone(),
                        actual,
                    });
                }
            }
            tokio::fs::rename(&part, &task.dest)
                .await
                .map_err(Error::io(&task.dest))
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&part).await;
        }
        result
    }
}

fn is_client_error(e: &Error) -> bool {
    matches!(e, Error::Http { source, .. } if source.status().is_some_and(|s| s.is_client_error()))
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

pub async fn sha1_file(path: &Path) -> Result<String> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut file = std::fs::File::open(&path).map_err(Error::io(&path))?;
        let mut hasher = Sha1::new();
        std::io::copy(&mut file, &mut hasher).map_err(Error::io(&path))?;
        Ok(format!("{:x}", hasher.finalize()))
    })
    .await
    .map_err(|e| Error::Invalid(format!("Hash-Task abgebrochen: {e}")))?
}
