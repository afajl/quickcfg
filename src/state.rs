//! Model for state file.

use failure::Error;
use fxhash::FxHasher64;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct Hashed {
    /// The last calculated hash.
    pub hash: u64,
    /// When it was last updated.
    pub updated: SystemTime,
}

/// The way the state is serialized.
#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Eq)]
pub struct DiskState {
    /// Last time git was updated.
    #[serde(default)]
    pub last_update: BTreeMap<String, SystemTime>,
    /// Things that should only happen once.
    #[serde(default)]
    pub once: BTreeMap<String, SystemTime>,
    #[serde(default)]
    pub hashes: BTreeMap<String, Hashed>,
}

impl DiskState {
    /// Convert into a state.
    pub fn to_state(self) -> State {
        State {
            dirty: false,
            last_update: self.last_update,
            once: self.once,
            hashes: self.hashes,
        }
    }
}

/// State model.
/// This keeps track of any changes with the dirty flag, which is an indication whether it should
/// be serialized or not.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct State {
    pub dirty: bool,
    /// Last time git was updated.
    pub last_update: BTreeMap<String, SystemTime>,
    /// Things that should only happen once.
    pub once: BTreeMap<String, SystemTime>,
    /// Things that have been tested against a hash.
    pub hashes: BTreeMap<String, Hashed>,
}

impl State {
    /// Get the last update timestamp for the given thing named `name`.
    pub fn last_update<'a>(&'a self, name: &str) -> Option<&'a SystemTime> {
        self.last_update.get(name)
    }

    /// Touch the thing with the given name.
    pub fn touch(&mut self, name: &str) {
        self.dirty = true;
        self.last_update.insert(name.to_string(), SystemTime::now());
    }

    /// Check if the given ID has run once.
    pub fn has_run_once(&self, id: &str) -> bool {
        self.once.contains_key(id)
    }

    /// Mark that something has happened once.
    pub fn touch_once(&mut self, id: &str) {
        self.dirty = true;
        self.once.insert(id.to_string(), SystemTime::now());
    }

    /// Touch the hashed item.
    pub fn is_hash_fresh<H: Hash>(&self, id: &str, hash: H) -> Result<bool, Error> {
        let hashed = match self.hashes.get(id) {
            Some(hashed) => hashed,
            None => return Ok(false),
        };

        let mut state = FxHasher64::default();
        hash.hash(&mut state);
        Ok(hashed.hash == state.finish())
    }

    /// Touch the hashed item.
    pub fn touch_hash<H: Hash>(&mut self, id: &str, hash: H) -> Result<(), Error> {
        let mut state = FxHasher64::default();
        hash.hash(&mut state);

        self.dirty = true;

        self.hashes.insert(
            id.to_string(),
            Hashed {
                hash: state.finish(),
                updated: SystemTime::now(),
            },
        );

        Ok(())
    }

    /// Extend this state with another.
    pub fn extend(&mut self, other: State) {
        // nothing to extend.
        if !other.dirty {
            return;
        }

        self.dirty = true;
        self.last_update.extend(other.last_update);
        self.once.extend(other.once);
    }

    /// Serialize the state, returning `None` unless it is dirty.
    pub fn serialize(self) -> Option<DiskState> {
        if !self.dirty {
            return None;
        }

        Some(DiskState {
            last_update: self.last_update,
            once: self.once,
            hashes: self.hashes,
        })
    }
}
