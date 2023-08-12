use std::error::Error;
use std::fs;
use std::io::BufWriter;
use std::num::NonZeroU32;

use fast_image_resize as fr;
use image::{ColorType, ImageEncoder};
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;

const IMAGE_EXTENSIONS: [&str; 108] = ["3dv", "ai", "amf", "art", "ase", "awg", "blp", "bmp", "bw", "cd5", "cdr", "cgm", "cit", "cmx", "cpt",
	"cr2", "cur", "cut", "dds", "dib", "djvu", "dxf", "e2d", "ecw", "egt", "emf", "eps", "exif", "gbr",
	"gif", "gpl", "grf", "hdp", "icns", "ico", "iff", "int", "inta", "jfif", "jng", "jp2", "jpeg", "jpg", "jps",
	"jxr", "lbm", "liff", "max", "miff", "mng", "msp", "nitf", "nrrd", "odg", "ota", "pam", "pbm", "pc1", "pc2",
	"pc3", "pcf", "pct", "pcx", "pdd", "pdn", "pgf", "pgm", "pi1", "pi2", "pi3", "pict", "png", "pnm", "pns",
	"ppm", "psb", "psp", "px", "pxm", "pxr", "qfx", "ras", "raw", "rgb", "rgba", "rle", "sct", "sgi",
	"sid", "stl", "sun", "svg", "sxd", "tga", "tif", "tiff", "v2d", "vnd", "vrml", "vtf", "wdp", "webp", "wmf",
	"x3d", "xar", "xbm", "xcf", "xpm"];

pub(crate) fn create_thumbnail(base_dir: &String, src_img_path: &String, thumbnail_size: &String)
							   -> Result<String, Box<dyn Error>> {
	let new_height: u32;
	if "small".eq(thumbnail_size) {
		new_height = 320;
	} else {
		new_height = 720;
	}
	let relative_path = &src_img_path[base_dir.len() + 1..src_img_path.len()];
	if relative_path.starts_with(".thumbnails-") {
		return Ok(src_img_path.to_string());
	}
	let mut target_img_path = String::new();
	target_img_path.push_str(base_dir);
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
	let img = match rotation {
		0 => img,
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
				return if rotation.eq("Straight") {
					0
				} else if rotation.eq("Rotated to left") {
					90
				} else if rotation.eq("Rotated to right") {
					270
				} else {
					180
				};
			}
		}
	}
	return 0;
}

pub(crate) fn is_image(file_extension: &String) -> bool {
	for extension in IMAGE_EXTENSIONS {
		if file_extension.eq(extension) {
			return true;
		}
	}
	return false;
}
