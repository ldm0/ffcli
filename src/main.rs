// For the `&raw *` used in the macro of options.rs, will be stabilized later
#![feature(raw_ref_op)]
// For the half open range in match in `split_commandline()`'s AVOption part
#![feature(exclusive_range_pattern)]
#![feature(half_open_range_patterns)]
mod cmdutils;
mod ffmpeg;
mod ffmpeg_opt;
mod options;

use env_logger;

use ffmpeg::ffmpeg;
fn main() {
    env_logger::init();
    ffmpeg();
}
