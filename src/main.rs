use std::fs;

use actix_files::NamedFile;
use actix_web::{App, get, HttpResponse, HttpServer, Responder, web};
use rust_embed::RustEmbed;
use serde::{Deserialize};

mod imageutils;
mod textutils;
mod fileutils;


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

#[derive(Deserialize)]
struct ListQuery {
	path: String,
}

#[get("list")]
async fn list(query: web::Query<ListQuery>, app_state: web::Data<AppState>)
			  -> HttpResponse {
	let absolute_dir_path = app_state.generate_absolute_path(&query.path);
	match fileutils::list(&absolute_dir_path) {
		Ok(list) => HttpResponse::Ok().content_type("application/json").json(list),
		Err(error) =>  HttpResponse::from_error(error)
	}
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
	let file = match imageutils::create_thumbnail(&app_state, &final_path, &query.thumbnail_size) {
		Ok(target_file) => {
			target_file
		}
		Err(error) => {
			eprintln!("Problem while creating thumbnail of image {}, ERROR - {:?}", &final_path, error);
			final_path
		}
	};
	let named_file = NamedFile::open_async(file).await?;
	Ok(named_file)
}





