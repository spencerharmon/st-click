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

    /// How many bars long the named sequence is. Defaults to 1 when the
    /// sequence is in the legacy list form (no `bars:` key possible).
    /// In the new mapping form, reads the optional `bars:` field.
    pub fn sequence_bars(&self, seq_name: &str) -> u32 {
	bars_from_yaml(&self.yaml[seq_name])
    }

    pub fn apply_sequence(self, seq: &mut Sequence, seq_name: String) {
	let node = &self.yaml[seq_name.as_str()];
	let yaml_vec = notes_from_yaml(node)
	    .expect("sequence name not found or has no notes");
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
	    let mut offset: f32 = 0.0;
	    if hash.contains_key(&yaml::Yaml::String("offset".to_string())) {
		// YAML may give us either a float (`1.5`) or an int (`2`);
		// accept both transparently.
		let raw = &note["offset"];
		offset = raw
		    .as_f64()
		    .or_else(|| raw.as_i64().map(|i| i as f64))
		    .expect("offset present but not numeric") as f32;
	    }
	    seq.add_notes_with_offset(
		note_utils::get_bytes_for_note_str(
		    note["note"].as_str().expect("Note string absent or invalid").to_string()
		),
		every,
		skip,
		beat_value,
		offset,
	    );
	}
    }
}

/// Extract the notes list from a sequence node, handling both schema
/// forms transparently:
///   - List form (legacy): `seq_name: [ <note>, <note>, ... ]`
///   - Mapping form:       `seq_name: { bars: N, notes: [ ... ] }`
fn notes_from_yaml(node: &yaml::Yaml) -> Option<&Vec<yaml::Yaml>> {
    if let Some(v) = node.as_vec() {
	return Some(v);
    }
    if let Some(h) = node.as_hash() {
	let key = yaml::Yaml::String("notes".to_string());
	if let Some(notes) = h.get(&key) {
	    return notes.as_vec();
	}
    }
    None
}

/// Read the optional `bars:` field from a mapping-form sequence node.
/// Returns 1 when absent or when the node is in legacy list form.
fn bars_from_yaml(node: &yaml::Yaml) -> u32 {
    if let Some(h) = node.as_hash() {
	let key = yaml::Yaml::String("bars".to_string());
	if let Some(b) = h.get(&key) {
	    if let Some(n) = b.as_i64() {
		if n > 0 {
		    return n as u32;
		}
	    }
	}
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Config {
	let docs = YamlLoader::load_from_str(yaml).unwrap();
	Config { yaml: docs[0].to_owned() }
    }

    #[test]
    fn list_form_implies_one_bar() {
	let cfg = parse("backbeat:\n  - { note: C-1, beat_value: 1.0 }\n");
	assert_eq!(cfg.sequence_bars("backbeat"), 1);
    }

    #[test]
    fn mapping_form_without_bars_defaults_to_one() {
	let cfg = parse("seq:\n  notes:\n    - { note: C-1, beat_value: 1.0 }\n");
	assert_eq!(cfg.sequence_bars("seq"), 1);
    }

    #[test]
    fn mapping_form_with_bars_is_read() {
	let cfg = parse("four_bar:\n  bars: 4\n  notes:\n    - { note: C-1, beat_value: 1.0 }\n");
	assert_eq!(cfg.sequence_bars("four_bar"), 4);
    }

    #[test]
    fn bars_zero_or_negative_falls_back_to_one() {
	let cfg = parse("bad:\n  bars: 0\n  notes:\n    - { note: C-1, beat_value: 1.0 }\n");
	assert_eq!(cfg.sequence_bars("bad"), 1);
	let cfg = parse("bad2:\n  bars: -3\n  notes:\n    - { note: C-1, beat_value: 1.0 }\n");
	assert_eq!(cfg.sequence_bars("bad2"), 1);
    }

    #[test]
    fn sequence_names_include_both_forms() {
	let cfg = parse(
	    "list_form:\n  - { note: C-1, beat_value: 1.0 }\n\
	     map_form:\n  bars: 2\n  notes:\n    - { note: D-1, beat_value: 1.0 }\n",
	);
	let mut names = cfg.sequence_names();
	names.sort();
	assert_eq!(names, vec!["list_form".to_string(), "map_form".to_string()]);
    }
}
