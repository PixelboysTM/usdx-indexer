use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    songs: PathBuf,

    /// Number of times to greet
    #[arg(short, long)]
    out_file: PathBuf,
}

#[derive(serde::Serialize, Default)]
struct Song {
    title: String,
    artist: Vec<String>,
    duration: u64,
    tags: Vec<String>,
    cover_image: String, // Base64 encoded image with data URI scheme
    bpm: u64,
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

fn parse_song(path: &Path) -> Option<Song> {
    let mut song = None;
    let contents = std::fs::read_dir(path).ok()?;
    for content in contents {
        let content = content.ok()?;
        if content.file_name().to_string_lossy().ends_with(".txt") {
            let text = std::fs::read_to_string(content.path()).ok()?;
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
                        // let cover_path = path.join(tags[1].trim());
                        // if let Some(base64_img) = encode_image_to_base64(&cover_path) {
                        //     s.cover_image = base64_img;
                        // }
                    }
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
                s.title = content.file_name().to_string_lossy().to_string();
            }

            song = Some(s);
            break;
        }
    }

    song
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
            return Some(total_seconds);
        }
    }

    None
}

fn main() {
    let args = Args::parse();

    let mut data = vec![];
    let songs = std::fs::read_dir(args.songs).expect("Must be a dir");
    for song in songs {
        if let Ok(song) = song {
            if song.file_type().unwrap().is_dir() {
                if let Some(s) = parse_song(&song.path()) {
                    println!("Indexed {}", s.title);
                    data.push(s);
                } else {
                    println!("Skipping {:?}", song);
                }
            }
        }
    }

    std::fs::write(args.out_file, serde_json::to_string(&data).unwrap()).unwrap();
}

/*
#VERSION:1.0.0
#TITLE:Tubthumping
#ARTIST:Chumbawamba
#LANGUAGE:English
#EDITION:SingStar ’90s [US], SingStar Summer Party [DE]
#YEAR:1997
#CREATOR:Leki
#MP3:Chumbawamba - Tubthumping.m4a
#COVER:Chumbawamba - Tubthumping [CO].jpg
#BACKGROUND:Chumbawamba - Tubthumping [BG].jpg
#VIDEO:Chumbawamba - Tubthumping.mp4
#BPM:340
#GAP:800


*/
