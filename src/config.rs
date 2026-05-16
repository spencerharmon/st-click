extern crate yaml_rust;
use yaml_rust::*;
use crate::sequencer::Sequence;
use crate::note_utils;
use crate::beat_values::tuplet;
use st_lib::config::find_config;
pub struct Config {
    yaml: yaml::Yaml
}

impl Config {
    pub fn new() -> Config {
	let path = find_config("st-click").expect("no configuration found");
	let s = std::fs::read_to_string(&path).expect("failed to read config file");
	let docs = YamlLoader::load_from_str(s.as_str()).unwrap();
	let yaml = &docs[0];
	println!("loaded config: {}", path.display());
	Config { yaml: yaml.to_owned() }
    }

    /// Top-level keys in the loaded YAML — i.e. the list of sequence
    /// names the user can pick. Returned in insertion order (yaml-rust
    /// preserves Hash key order).
    pub fn sequence_names(&self) -> Vec<String> {
	let mut out = Vec::new();
	if let Some(h) = self.yaml.as_hash() {
	    for (k, _) in h {
		if let Some(s) = k.as_str() {
		    out.push(s.to_string());
		}
	    }
	}
	out
    }
	pub fn apply_sequence(self, seq: &mut Sequence, seq_name: String) {
	let yaml_vec = self.yaml[seq_name.as_str()].as_vec().expect("sequence name not found");
	for i in 0..yaml_vec.len() {
	    let note = &yaml_vec[i];
	    let hash = note.as_hash().unwrap();
	    let mut beat_value = note["beat_value"].as_f64().expect("beat value absent or invalid") as f32;
	    if hash.contains_key(&yaml::Yaml::String("tuplet".to_string())) {
		beat_value = tuplet(beat_value, note["tuplet"].as_i64().unwrap() as u16);
	    }
	    let mut every: u16 = 1;
	    if hash.contains_key(&yaml::Yaml::String("every".to_string())) {
		every = note["every"].as_i64().unwrap() as u16;
	    }
	    let mut skip: u16 = 0;
	    if hash.contains_key(&yaml::Yaml::String("skip".to_string())) {
		skip = note["skip"].as_i64().unwrap() as u16;
	    }	    
	    seq.add_notes(
		note_utils::get_bytes_for_note_str(
		    note["note"].as_str().expect("Note string absent or invalid").to_string()
		),
		every,
		skip,
		beat_value
	    );
	}
    }
}
