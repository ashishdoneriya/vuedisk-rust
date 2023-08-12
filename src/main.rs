use std::error::Error;
use std::fs;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::os::unix::fs::MetadataExt;

use actix_files::NamedFile;
use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};
use fast_image_resize as fr;
use image::{ColorType, ImageEncoder};
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
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
	}).keep_alive(None)
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
	fs::metadata(&final_path)?;
	let file = match create_thumbnail(&app_state, &final_path, &query.thumbnail_size) {
		Ok(target_file) => {
			target_file
		},
		Err(error) => {
			eprintln!("Problem while creating thumbnail of image {}, ERROR - {:?}", &final_path, error);
			final_path
		}
	};
	let named_file = NamedFile::open_async(file).await?;
	Ok(named_file)
}


fn create_thumbnail(app_state: &web::Data<AppState>, src_img_path: &String, thumbnail_size: &String)
		-> Result<String, Box<dyn Error>> {
	let new_height: u32;
	if "small".eq(thumbnail_size) {
		new_height = 320;
	} else {
		new_height = 720;
	}
	let relative_path = &src_img_path[app_state.base_dir.len() + 1..src_img_path.len()];
	if relative_path.starts_with(".thumbnails-") {
		return Ok(src_img_path.to_string());
	}
	let mut target_img_path = String::new();
	target_img_path.push_str(&app_state.base_dir);
	target_img_path.push(std::path::MAIN_SEPARATOR);
	target_img_path.push_str(&".thumbnails-height-");
	target_img_path.push_str(&new_height.to_string());
	target_img_path.push(std::path::MAIN_SEPARATOR);
	target_img_path.push_str(relative_path);
	if fs::metadata(&target_img_path).is_ok() {
		return Ok(target_img_path);
	}
	if let Some(parent_dir) = std::path::Path::new(&target_img_path).parent() {
		if !parent_dir.exists() {
			fs::create_dir_all(parent_dir)?;
		}
	}

	match resize(&src_img_path, &target_img_path, new_height) {
		Ok(_) => Ok(target_img_path),
		Err(error) => {
			println!("Couldn't generate thumbnail of {:?}, error - {:?}", src_img_path, error);
			let target_file = std::path::Path::new(&target_img_path);
			let src_file = std::path::Path::new(&src_img_path);
			symlink::symlink_file(src_file, target_file)?;
			Ok(target_img_path)
		}
	}
}

fn resize(src_img_path: &String, target_img_path: &String, new_height: u32) -> Result<(), Box<dyn Error>> {
	let img = ImageReader::open(src_img_path)?.decode()?;
	let rotation = get_image_rotation(src_img_path);
	let img = match rotation { 0 => img,
		90 => img.rotate90(),
		180 => img.rotate180(),
		270 => img.rotate270(),
		_ => img
	};
	let old_height = img.height();
	if new_height > old_height {
		let target_file = std::path::Path::new(&target_img_path);
		let src_file = std::path::Path::new(&src_img_path);
		symlink::symlink_file(src_file, target_file)?;
		return Ok(());
	}
	let old_width = img.width();
	let img_width = NonZeroU32::new(old_width).ok_or("Couldn't create NonZeroU32 for old_width")?;

	let img_height = NonZeroU32::new(old_height).ok_or("Couldn't create NonZeroU32 for old_height")?;
	let mut src_image = fr::Image::from_vec_u8(
		img_width,
		img_height,
		img.to_rgba8().into_raw(),
		fr::PixelType::U8x4,
	)?;

	// Multiple RGB channels of source image by alpha channel
	// (not required for the Nearest algorithm)
	let alpha_mul_div = fr::MulDiv::default();
	alpha_mul_div.multiply_alpha_inplace(&mut src_image.view_mut())?;

	let new_width = (old_width * new_height) / old_height;

	// Create container for data of destination image
	let dst_width = NonZeroU32::new(new_width).ok_or("Couldn't create NonZeroU32 for new_width")?;
	let dst_height = NonZeroU32::new(new_height).ok_or("Couldn't create NonZeroU32 for new_height")?;
	let mut dst_image = fr::Image::new(dst_width, dst_height, src_image.pixel_type());

	// Get mutable view of destination image data
	let mut dst_view = dst_image.view_mut();

	// Create Resizer instance and resize source image
	// into buffer of destination image
	let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
	resizer.resize(&src_image.view(), &mut dst_view)?;

	// Divide RGB channels of destination image by alpha
	alpha_mul_div.divide_alpha_inplace(&mut dst_view)?;

	// Write destination image
	let mut result_buf: BufWriter<Vec<u8>> = BufWriter::new(Vec::new());
	PngEncoder::new(&mut result_buf)
		.write_image(
			dst_image.buffer(),
			dst_width.get(),
			dst_height.get(),
			ColorType::Rgba8,
		)?;

	// Save the result to the destination path
	let result_content = result_buf.into_inner()?;
	fs::write(target_img_path, &result_content)?;
	Ok(())
}

fn get_image_rotation(file_path: &String) -> u32 {
	if let Ok(exif) = rexif::parse_file(file_path) {
		for entry in &exif.entries {
			if entry.tag.to_string().eq("Orientation") {
				let rotation = entry.value_more_readable.to_string();
				if rotation.eq("Straight") {
					return 0;
				} else if rotation.eq("Rotated to left") {
					return 90;
				} else if rotation.eq("Rotated to right") {
					return 270;
				} else {
					return 180;
				}
			}
		}
	}
	return 0;
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


