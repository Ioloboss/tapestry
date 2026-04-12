use std::{path::Path, time::Instant};

use tapestry::font::{Font, Mapping};

fn main() {
	let filename = Path::new("./resources/fonts/Noto_Sans_JP/static/NotoSansJP-Regular.ttf");

	let before = Instant::now();
	let font = Font::new(filename);
	let elapsed_time = before.elapsed();
	println!("Loading Font took {} milliseconds", elapsed_time.as_millis());
	println!("Font has {} glyphs", font.glyphs.len());
	println!("So font takes {} milliseconds per glyph and does {} glyphs per second", elapsed_time.as_millis() as f64 / font.glyphs.len() as f64, font.glyphs.len() as f64 / elapsed_time.as_secs() as f64 );
	println!("Has Mappings:");
	for mapping in font.mappings.iter() {
		println!("	Mapping Format: {}",
			match mapping {
				Mapping::TrueTypeFormat12(_) => "12".to_string(),
				Mapping::TrueTypeFormat4(_) => "4".to_string(),
				Mapping::InvalidFormat(format) => format!("Invalid: {format}"),
			}
		);
	}
}