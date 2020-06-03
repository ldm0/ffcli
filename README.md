# FFCLI

This project is meant to build FFmpeg command line arguments parser. This is roughly a ported ffmpeg.

`AVOptions` have not been supported.

You can test by using `cargo test`. And you can use `RUST_LOG=debug cargo run -- -i input.mp4 ouput.mkv` to check program outputs (I changed the `av_log`s in FFmpeg's source code to `log::{debug, error}'). So you will see the exact same debug message when specific parameters are given.

It's under the original license of FFmpeg.