mod cmdutils;
mod commands;
mod ffmpeg;
mod ffmpeg_opt;

use env_logger;

use ffmpeg::ffmpeg;
fn main() {
    env_logger::init();
    ffmpeg();
}
