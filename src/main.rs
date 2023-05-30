use clap::Parser;
use hound::{SampleFormat, WavWriter};
use std::{
    io::{BufRead, BufReader},
    process::{Child, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.sec.is_none() && args.millisecond.is_none() {
        panic!("must be set sec or msec");
    }

    let secs = if args.sec.is_none() {
        (args.millisecond.unwrap() as f64 / 1000.0) as usize
    } else {
        args.sec.unwrap()
    };

    let rb_size = 44100 * secs;
    let mut rb = LocalRb::<u8, Vec<_>>::new(rb_size);
    let (mut prod, mut cons) = rb.split_ref();

    let now = Instant::now();
    let collect_time = Duration::from_secs(secs as u64);

    let (mut child, stdout) = get_yt_dlp_stdout(&args.url);
    let mut reader = BufReader::new(stdout);

    let mut process = || {
        let data = cons.pop_iter().collect::<Vec<u8>>();
        let audio_data = audio::get_audio_data(data.as_ref()).unwrap();
        let _ = write_wav_file(args.output.as_str(), audio_data.0.as_ref(), 22050, 1);
    };

    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            break;
        }

        let len = buf.len();
        prod.push_slice(buf);

        if now.elapsed() >= collect_time {
            process();
            break;
        }

        reader.consume(len);
    }

    child.kill().expect("failed to kill yt-dlp process");
    println!("wrote to {} file done.", args.output);
    Ok(())
}

fn get_yt_dlp_stdout(url: &str) -> (Child, ChildStdout) {
    let mut cmd = Command::new("yt-dlp");
    cmd.arg(url)
        .args(["-f", "w"])
        .args(["--quiet"])
        .args(["-o", "-"]);

    let mut child = cmd
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute yt-dlp");

    let stdout = child.stdout.take().expect("invalid stdout stream");

    (child, stdout)
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
