use std::error::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;

use serde::Serialize;

use crate::{imageutils, textutils};

const VIDEO_EXTENSIONS: [&str; 6] = ["avi", "mp4", "mpeg", "mpg", "ogg", "webm"];
const AUDIO_EXTENSIONS: [&str; 3] = ["aac", "mp3", "wav"];

#[derive(Serialize)]
pub struct FileListObj {
	pub name: String,
	#[serde(rename = "isDir")]
	pub is_dir: bool,
	size: String,
	#[serde(rename = "isHidden")]
	pub is_hidden: bool,
	#[serde(rename = "isText")]
	pub is_text: bool,
	#[serde(rename = "isImage")]
	pub is_image: bool,
	#[serde(rename = "isAudio")]
	pub is_audio: bool,
	#[serde(rename = "isVideo")]
	pub is_video: bool,
}

impl FileListObj {
	pub fn new_dir(dir_name: &String) -> FileListObj {
		FileListObj {
			name: dir_name.to_string(),
			is_dir: true,
			size: String::from("0 B"),
			is_hidden: false,
			is_text: false,
			is_image: false,
			is_audio: false,
			is_video: false,
		}
	}

	pub fn new_file(file_name: &String, file_size: u64) -> FileListObj {
		let file_extension = get_extension(&file_name);
		FileListObj {
			name: file_name.clone(),
			is_dir: false,
			size: get_size_in_string(file_size),
			is_hidden: false,
			is_text: textutils::is_text(&file_extension),
			is_image: imageutils::is_image(&file_extension),
			is_audio: is_audio(&file_extension),
			is_video: is_video(&file_extension),
		}
	}
}

pub fn get_extension(file_name: &str) -> String {
	match file_name.rfind('.') {
		Some(size) => file_name[size + 1..].to_lowercase(),
		None => "".to_string(),
	}
}

pub fn get_size_in_string(size: u64) -> String {
	if size > 1_000_000_000 {
		format!("{:.2} GB", size as f64 / 1_000_000_000.0)
	} else if size > 1_000_000 {
		format!("{} MB", size / 1_000_000)
	} else if size > 1_000 {
		format!("{} KB", size / 1_000)
	} else {
		format!("{} B", size)
	}
}

pub fn list(dir_path: &String) -> Result<Vec<FileListObj>, Box<dyn Error>> {
	let mut list = Vec::new();
	let files = fs::read_dir(dir_path)?;
	for file_result in files {
		if let Ok(file) = file_result {
			let metadata_result = file.metadata();
			if let Ok(metadata) = metadata_result {
				let file_name = file.file_name().to_string_lossy().to_string();
				if metadata.is_dir() {
					list.push(FileListObj::new_dir(&file_name))
				} else {
					list.push(FileListObj::new_file(&file_name, metadata.size()));
				}
			}
		}
	}
	Ok(list)
}


fn is_video(file_extension: &String) -> bool {
	for extension in VIDEO_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

fn is_audio(file_extension: &String) -> bool {
	for extension in AUDIO_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

