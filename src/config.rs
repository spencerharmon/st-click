extern crate yaml_rust;
use std::mem::MaybeUninit;
use yaml_rust::*;
use home;
use crate::sequencer::Sequence;
use crate::note_utils;
use crate::beat_values::tuplet;
pub struct Config {
    yaml: yaml::Yaml
}

impl Config {
    pub fn new() -> Config {
	let mut paths: Vec<String> = Vec::new();
	if let Some(home) = home::home_dir() {
	    paths.push(format!("{}/.config/st-tools/st-click.yaml", home.display()).to_string());
	}
	paths.push("/etc/st-tools/st-click.yaml".to_string());

	for p in paths {
	    match  std::fs::read_to_string(p) {
		Ok(s) => {
		    let docs = YamlLoader::load_from_str(s.as_str()).unwrap();
		    let yaml = &docs[0];
		    println!("{:?}", s);
		    return Config { yaml: yaml.to_owned() };
		}
		Err(_) => continue
	    }
	}
	panic!("no configuration found");
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
