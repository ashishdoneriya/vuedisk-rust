use std::error::Error;
use std::fs;
use std::os::unix::fs::MetadataExt;

use actix_files::NamedFile;
use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};
use image::GenericImageView;
use image::imageops::FilterType;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

const IMAGE_EXTENSIONS: [&str; 108] = ["3dv", "ai", "amf", "art", "ase", "awg", "blp", "bmp", "bw", "cd5", "cdr", "cgm", "cit", "cmx", "cpt",
	"cr2", "cur", "cut", "dds", "dib", "djvu", "dxf", "e2d", "ecw", "egt", "emf", "eps", "exif", "gbr",
	"gif", "gpl", "grf", "hdp", "icns", "ico", "iff", "int", "inta", "jfif", "jng", "jp2", "jpeg", "jpg", "jps",
	"jxr", "lbm", "liff", "max", "miff", "mng", "msp", "nitf", "nrrd", "odg", "ota", "pam", "pbm", "pc1", "pc2",
	"pc3", "pcf", "pct", "pcx", "pdd", "pdn", "pgf", "pgm", "pi1", "pi2", "pi3", "pict", "png", "pnm", "pns",
	"ppm", "psb", "psp", "px", "pxm", "pxr", "qfx", "ras", "raw", "rgb", "rgba", "rle", "sct", "sgi",
	"sid", "stl", "sun", "svg", "sxd", "tga", "tif", "tiff", "v2d", "vnd", "vrml", "vtf", "wdp", "webp", "wmf",
	"x3d", "xar", "xbm", "xcf", "xpm"];
const VIDEO_EXTENSIONS: [&str; 6] = ["avi", "mp4", "mpeg", "mpg", "ogg", "webm"];
const AUDIO_EXTENSIONS: [&str; 3] = ["aac", "mp3", "wav"];
const TEXT_EXTENSIONS: [&str; 113] = ["gnumakefile", "makefile", "ada", "adb", "ads", "ahk", "alg", "as", "ascx", "ashx", "asp", "aspx", "awk",
	"bash", "bat", "c", "cbl", "cc", "cfg", "cfm", "cfml", "clj", "cmf", "cob", "coffee", "config", "cpp",
	"cpy", "cs", "css", "csv", "cxx", "d", "dart", "e", "erl", "ex", "exs", "f", "f90", "f95", "fsx", "go",
	"groovy", "h", "hpp", "hrl", "hs", "htaccess", "htm", "html", "inc", "j", "jade", "java", "jl", "js",
	"json", "jsp", "kt", "liquid", "lisp", "log", "lsp", "lua", "m", "makefile", "md", "ml", "mli", "mm", "nim",
	"pas", "php", "pl", "pp", "prg", "pro", "properties", "ps1", "psm1", "pwn", "py", "r", "rb", "rkt", "rs",
	"rss", "sas", "sass", "scala", "scm", "scss", "sh", "sql", "st", "swift", "tcl", "text", "toml", "ts", "v",
	"vb", "vh", "vhd", "vhdl", "vm", "vue", "xml", "xsl", "xstl", "yaml", "zsh"];

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let app_state = AppState {
		base_dir: String::from("/home/ashish")
	};
	HttpServer::new(move || {
		App::new().app_data(web::Data::new(app_state.clone()))
			.service(
				web::scope("apis").service(list).service(thumbnail))
			.service(index)
			.service(dist)
	})
		.bind("0.0.0.0:8080")?
		.run()
		.await
}

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

#[get("/")]
async fn index() -> impl Responder {
	handle_embedded_file("index.html")
}

#[get("{tail:.*}")]
async fn dist(path: web::Path<String>) -> impl Responder {
	handle_embedded_file(path.as_str())
}

fn handle_embedded_file(path: &str) -> HttpResponse {
	match Asset::get(path) {
		Some(content) => HttpResponse::Ok()
			.content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
			.body(content.data.into_owned()),
		None => HttpResponse::NotFound().body("404 Not Found"),
	}
}


#[derive(Clone)]
struct AppState {
	base_dir: String,
}

impl AppState {
	fn generate_absolute_path(&self, parent_dir: &String) -> String {
		let mut absolute_path = String::from(&self.base_dir);
		absolute_path.push(std::path::MAIN_SEPARATOR);
		absolute_path.push_str(parent_dir);
		return absolute_path;
	}

	fn generate_absolute_path1(&self, parent_dir: &String, file_name: &String) -> String {
		let mut absolute_path = String::from(&self.base_dir);
		absolute_path.push(std::path::MAIN_SEPARATOR);
		absolute_path.push_str(parent_dir);
		absolute_path.push(std::path::MAIN_SEPARATOR);
		absolute_path.push_str(file_name);
		return absolute_path;
	}
}

#[get("list")]
async fn list(app_state: web::Data<AppState>, query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
	let params = &query.into_inner();
	let mut list = Vec::new();
	if let Some(child_dir) = params.get("path") {
		let absolute_dir_path = app_state.generate_absolute_path(child_dir);
		if let Ok(files) = fs::read_dir(absolute_dir_path) {
			for file_result in files {
				if let Ok(file) = file_result {
					let metadata_result = file.metadata();
					if let Ok(metadata) = metadata_result {
						let file_name = file.file_name().to_string_lossy().to_string();
						if metadata.is_dir() {
							list.push(FileListObj {
								name: file_name,
								is_dir: true,
								size: String::from("0 B"),
								is_hidden: false,
								is_text: false,
								is_image: false,
								is_audio: false,
								is_video: false,
							})
						} else {
							list.push(FileListObj {
								name: file_name.clone(),
								is_dir: false,
								size: get_size_in_string(metadata.size()),
								is_hidden: false,
								is_text: is_text(&file_name),
								is_image: is_image(&file_name),
								is_audio: is_audio(&file_name),
								is_video: is_video(&file_name),
							})
						}
					}
				}
			}
		}
	}
	web::Json(list)
}

#[derive(Deserialize)]
struct ThumbnailQuery {
	#[serde(rename = "parent")]
	source_dir_path: String,
	#[serde(rename = "type")]
	thumbnail_size: String,
	#[serde(rename = "name")]
	file_name: String,
}

#[get("thumbnail")]
async fn thumbnail(query: web::Query<ThumbnailQuery>, app_state: web::Data<AppState>) -> actix_web::Result<NamedFile> {
	let final_path = app_state.generate_absolute_path1(&query.source_dir_path, &query.file_name);
	println!("{}", final_path);
	fs::metadata(&final_path)?;
	let file = match create_thumbnail(&app_state, &final_path, &query.thumbnail_size) {
		Ok(Some(target_file)) => {
			target_file
		}
		Ok(None) => {
			final_path
		}
		Err(error) => {
			eprintln!("Problem while creating thumbnail of image {}, ERROR - {:?}", &final_path, error);
			final_path
		}
	};
	let named_file = NamedFile::open_async(file).await?;
	Ok(named_file)
}

fn create_thumbnail(app_state: &web::Data<AppState>, file_path: &String, thumbnail_size: &String) -> Result<Option<String>, Box<dyn Error>> {
	let height: u32;
	if "small".eq(thumbnail_size) {
		height = 320;
	} else {
		height = 720;
	}
	let relative_path = &file_path[app_state.base_dir.len() + 1..file_path.len()];
	if relative_path.starts_with(".thumbnails-") {
		return Ok(Some(file_path.to_string()));
	}
	let mut target_path = String::new();
	target_path.push_str(&app_state.base_dir);
	target_path.push(std::path::MAIN_SEPARATOR);
	target_path.push_str(&".thumbnails-height-");
	target_path.push_str(&height.to_string());
	target_path.push(std::path::MAIN_SEPARATOR);
	target_path.push_str(relative_path);
	if fs::metadata(&target_path).is_ok() {
		return Ok(Some(target_path));
	}
	if let Some(parent_dir) = std::path::Path::new(&target_path).parent() {
		if !parent_dir.exists() {
			fs::create_dir_all(parent_dir)?;
		}
	}
	let img = image::open(&target_path)?;
	let (img_width, img_height) = img.dimensions();
	if img_height <= height {
		let target = std::path::Path::new(&target_path);
		let file = std::path::Path::new(&file_path);
		symlink::symlink_file(target, file)?;
		return Ok(Some(target_path));
	}
	let width = (img_width * height) / img_height;
	if let Err(error) = img.resize(width, height, FilterType::CatmullRom).save(&target_path) {
		eprintln!("Problem while creating thumbnail of image {}, ERROR - {:?}", file_path, error);
		let target = std::path::Path::new(&target_path);
		let file = std::path::Path::new(&file_path);
		symlink::symlink_file(target, file)?;
	}
	return Ok(Some(target_path));
}

fn is_image(file_name: &String) -> bool {
	let file_extension = get_extension(file_name);
	for extension in IMAGE_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

fn is_video(file_name: &String) -> bool {
	let file_extension = get_extension(file_name);
	for extension in VIDEO_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

fn is_audio(file_name: &String) -> bool {
	let file_extension = get_extension(file_name);
	for extension in AUDIO_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

fn is_text(file_name: &String) -> bool {
	let file_extension = get_extension(file_name);
	for extension in TEXT_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}

fn get_extension(file_name: &str) -> String {
	match file_name.rfind('.') {
		Some(size) => file_name[size + 1..].to_lowercase(),
		None => "".to_string(),
	}
}

fn get_size_in_string(size: u64) -> String {
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

#[derive(Serialize)]
struct FileListObj {
	name: String,
	#[serde(rename = "isDir")]
	is_dir: bool,
	size: String,
	#[serde(rename = "isHidden")]
	is_hidden: bool,
	#[serde(rename = "isText")]
	is_text: bool,
	#[serde(rename = "isImage")]
	is_image: bool,
	#[serde(rename = "isAudio")]
	is_audio: bool,
	#[serde(rename = "isVideo")]
	is_video: bool,
}


