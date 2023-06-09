use clap::Parser;
use hound::{SampleFormat, WavWriter};
use rusty_ytdl::{VideoOptions, VideoQuality, VideoSearchOptions, Video};

use ringbuf::LocalRb;

mod audio;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// recode size of second
    #[arg(short, long)]
    sec: Option<usize>,

    /// recode size of millisecond
    #[arg(short, long)]
    millisecond: Option<usize>,

    /// youtube url or youtube video id
    #[arg()]
    url: String,

    /// wav output file path
    #[arg(short, long)]
    output: String,
}

#[tokio::main()]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.sec.is_none() && args.millisecond.is_none() {
        panic!("must be set sec or msec");
    }

    let secs = if args.sec.is_none() {
        (args.millisecond.unwrap() as f64 / 1000.0) as usize
    } else {
        args.sec.unwrap()
    };

    let rb_size = 22050 * secs;
    let mut rb = LocalRb::<f32, Vec<_>>::new(rb_size);
    let (mut prod, mut cons) = rb.split_ref();

    let video_options = VideoOptions {
        quality: VideoQuality::Lowest,
        filter: VideoSearchOptions::VideoAudio,
        ..Default::default()
    };

    let video = Video::new_with_options(args.url, video_options).unwrap();
    let stream = video.stream().await.unwrap();

    while let Some(chunk) = stream.chunk().await.unwrap() {
        let (data, _) = audio::get_audio_data(chunk.as_ref()).unwrap();
        println!("get {}kb and {}kb audio data", chunk.len() / 1024, data.len() / 1024);
        prod.push_slice(data.as_ref());

        if prod.is_full() {
            let data = cons.pop_iter().collect::<Vec<f32>>();
            let _ = write_wav_file(args.output.as_str(), data.as_ref(), 22050, 1);
            break;
        }
    }

    println!("wrote to {} file done.", args.output);
    Ok(())
}

fn write_wav_file(
    file_path: &str,
    audio_data: &[f32],
    sample_rate: u32,
    num_channels: u16,
) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: num_channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(file_path, spec)?;

    for &sample in audio_data {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(())
}
