// For the `&raw *` used in the macro of options.rs, will be stabilized later
#![feature(raw_ref_op)]
mod cmdutils;
mod options;
mod ffmpeg;
mod ffmpeg_opt;

use env_logger;

use ffmpeg::ffmpeg;
fn main() {
    env_logger::init();
    ffmpeg();
}
