use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Where the songs are located
    #[arg(short, long, required = true)]
    songs: Vec<PathBuf>,

    /// Where to put the output json file e.g. out.json
    #[arg(short, long)]
    out_file: PathBuf,

    /// Where to store the cover images
    #[arg(short, long)]
    cover_dir: Option<PathBuf>,
}

#[derive(serde::Serialize, Default)]
struct Song {
    title: String,
    artist: Vec<String>,
    duration: u64,
    tags: Vec<String>,
    cover_image: String, // Base64 encoded image with data URI scheme
    bpm: u64,
    gap: u64,
}

fn encode_image_to_base64(path: &Path) -> Option<String> {
    let img = image::open(path).ok()?;

    let mut buffer = Vec::new();
    img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
        .ok()?;

    // Create data URI with base64 encoded PNG
    let base64_string = BASE64.encode(buffer);
    Some(format!("data:image/png;base64,{}", base64_string))
}

fn parse_song(path: &Path, cover_index: &mut u64, cover_dir: Option<&PathBuf>) -> Result<Song, Box<dyn std::error::Error>> {
    let mut song = None;
    let contents = std::fs::read_dir(path)?;
    for content in contents {
        let content = content?;
        if content.file_name().to_string_lossy().ends_with(".txt") {

            let file = std::fs::File::open(content.path())?;
            let data = file.bytes().collect::<Result<Vec<_>, _>>()?;
            let text = String::from_utf8_lossy(&data);

            let mut s = Song::default();
            for l in text.lines() {
                if !l.starts_with("#") {
                    continue;
                }
                let tags = l[1..].split(':').collect::<Vec<_>>();
                match tags[0] {
                    "TITLE" => s.title = tags[1].to_string(),
                    "ARTIST" => s.artist.push(tags[1].to_string()),
                    "COVER" => {
                        let cover_path = path.join(tags[1].trim());
                        if let Some(cd) = cover_dir {
                            let cn = format!("cover-{}.{}", cover_index, cover_path.to_string_lossy().split('.').last().unwrap_or(""));
                            std::fs::copy(&cover_path, cd.join(&cn))?;
                            s.cover_image = cn;
                            *cover_index += 1;
                        }
                        // if let Some(base64_img) = encode_image_to_base64(&cover_path) {
                        //     s.cover_image = base64_img;
                        // }
                    }
                    "GAP" =>s.gap = tags[1].replace(',', ".").trim().parse::<f32>().map_err(|e| eprintln!("{e}: {path:?}")).unwrap_or(0.0) as _,
                    "BPM" => s.bpm = tags[1].replace(',', ".").trim().parse::<f32>().map_err(|e| eprintln!("{e}: {path:?}")).unwrap_or(0.0) as _,
                    "END" => s.duration = (tags[1].parse::<u128>().unwrap_or(0) / 1000) as u64,
                    _ => {}
                }
            }

            if let Some(d) = try_fix_duration(&s, &text) {
                s.duration = d;
            }

            if s.title.is_empty() {
                println!("Title empty taking folder name");
                s.title = content.file_name().to_string_lossy().split('.').collect::<Vec<_>>()[0].to_string();
            }

            if s.artist.is_empty() {
                s.artist.push("".to_string());
            }

            song = Some(s);
            break;
        }
    }

    song.ok_or("Something went wrong".into())
}

fn try_fix_duration(s: &Song, text: &str) -> Option<u64> {
    if s.duration == 0 && s.bpm != 0 {
        if let Some(l) = text.lines().rev().find(|l| {
            [':', '*', 'R', 'F', 'G']
                .iter()
                .any(|s| (*l).starts_with(*s))
        }) {
            let l = l.split(' ').collect::<Vec<_>>();
            let start = l[1].parse::<u64>().ok()?;
            let dur = l[2].parse::<u64>().ok()?;
            let end = start + dur;
            let spb = 60.0 / s.bpm as f32;
            let total_seconds = (end as f32 * spb) as u64;
            return Some(total_seconds / 4 + (s.gap / 1000));
        }
    }

    None
}

fn main() {
    let args = Args::parse();

    let mut cover_index = 0;
    if let Some(d) = &args.cover_dir {
        std::fs::create_dir_all(d).unwrap();
    }

    let mut data = vec![];
    for lib in args.songs {
        let songs = std::fs::read_dir(lib).expect("Must be a dir");
        for song in songs.flatten() {
            if song.file_type().unwrap().is_dir() {
                match parse_song(&song.path(), &mut cover_index, args.cover_dir.as_ref()) {
                    Ok(s) => {println!("Indexed {}", s.title);
                    data.push(s);}
                    Err(e) => eprintln!("{song:?} {e:?}"),
                }
            }

        }
    }

    std::fs::write(args.out_file, serde_json::to_string(&data).unwrap()).unwrap();
}
