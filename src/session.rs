//! Tiny YAML session file for st-click.
//!
//! Persisted as `<nsm_path>/click.yaml`. Stores the currently selected
//! sequence name from the YAML config. The config file itself is not
//! copied into the session — st-click continues to read it from the
//! standard XDG location (see `st_lib::config::find_config`) so that
//! shared rhythms stay shared. If per-session config files become a
//! requirement, copy `find_config("st-click")` into the session dir on
//! Save and prefer that copy on Load.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
	pub sequence_name: String,
}

fn session_file(path: &str) -> PathBuf {
	let mut p = PathBuf::from(path);
	if !p.exists() {
		let _ = std::fs::create_dir_all(&p);
	}
	p.push("click.yaml");
	p
}

pub fn load(path: &str) -> Result<Option<Session>, Box<dyn std::error::Error>> {
	let p = session_file(path);
	if !p.exists() {
		return Ok(None);
	}
	let f = File::open(&p)?;
	let s: Session = serde_yaml::from_reader(f)?;
	Ok(Some(s))
}

pub fn save(path: &str, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
	let p = session_file(path);
	let mut f = File::create(&p)?;
	f.write_all(serde_yaml::to_string(session)?.as_bytes())?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::env::temp_dir;

	#[test]
	fn roundtrip() {
		let dir = temp_dir().join(format!("st-click-test-{}", std::process::id()));
		let s = Session { sequence_name: "quarters".to_string() };
		save(dir.to_str().unwrap(), &s).unwrap();
		let loaded = load(dir.to_str().unwrap()).unwrap().unwrap();
		assert_eq!(loaded.sequence_name, "quarters");
		std::fs::remove_dir_all(&dir).ok();
	}

	#[test]
	fn missing_returns_none() {
		let dir = temp_dir().join(format!("st-click-missing-{}", std::process::id()));
		assert!(load(dir.to_str().unwrap()).unwrap().is_none());
		std::fs::remove_dir_all(&dir).ok();
	}
}
