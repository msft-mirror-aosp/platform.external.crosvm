// Copyright 2024 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// TODO: remove in next change.
#![allow(dead_code)]

use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use crate::RutabagaError;
use crate::RutabagaResult;

pub struct RutabagaSnapshotWriter {
    dir: PathBuf,
}

impl RutabagaSnapshotWriter {
    pub fn from_existing(directory: PathBuf) -> Self {
        Self { dir: directory }
    }

    pub fn get_path(&self) -> PathBuf {
        self.dir.clone()
    }

    pub fn add_namespace(&self, name: &str) -> RutabagaResult<Self> {
        let directory = self.dir.join(name);

        std::fs::create_dir(&directory).map_err(RutabagaError::IoError)?;

        Ok(Self::from_existing(directory))
    }

    pub fn add_fragment<T: serde::Serialize>(&self, name: &str, t: &T) -> RutabagaResult<()> {
        let fragment_path = self.dir.join(name);
        let fragment_file = File::options()
            .write(true)
            .create_new(true)
            .open(fragment_path)
            .map_err(|e| {
                RutabagaError::SnapshotError(format!("failed to add fragment {}: {}", name, e))
            })?;
        let mut fragment_writer = BufWriter::new(fragment_file);
        serde_json::to_writer(&mut fragment_writer, t).map_err(|e| {
            RutabagaError::SnapshotError(format!("failed to write fragment {}: {}", name, e))
        })?;
        fragment_writer.flush().map_err(|e| {
            RutabagaError::SnapshotError(format!("failed to flush fragment {}: {}", name, e))
        })?;
        Ok(())
    }
}

pub struct RutabagaSnapshotReader {
    dir: PathBuf,
}

impl RutabagaSnapshotReader {
    pub fn new(directory: PathBuf) -> RutabagaResult<Self> {
        if !directory.as_path().exists() {
            return Err(RutabagaError::SnapshotError(format!(
                "{} does not exist",
                directory.display()
            )));
        }

        Ok(Self { dir: directory })
    }

    pub fn get_path(&self) -> PathBuf {
        self.dir.clone()
    }

    pub fn get_namespace(&self, name: &str) -> RutabagaResult<Self> {
        let directory = self.dir.join(name);
        Self::new(directory)
    }

    pub fn get_fragment<T: serde::de::DeserializeOwned>(&self, name: &str) -> RutabagaResult<T> {
        let fragment_path = self.dir.join(name);
        let fragment_file = File::open(fragment_path).map_err(|e| {
            RutabagaError::SnapshotError(format!("failed to get fragment {}: {}", name, e))
        })?;
        let mut fragment_reader = BufReader::new(fragment_file);
        serde_json::from_reader(&mut fragment_reader).map_err(|e| {
            RutabagaError::SnapshotError(format!("failed to read fragment {}: {}", name, e))
        })
    }
}
