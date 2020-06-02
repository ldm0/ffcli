# FFCLI

This project is meant to build FFmpeg command line arguments parser. This is roughly a port of `ffmpeg_parse_options()`.

Currently it only support the arguments of `CMDUTILS_COMMON_OPTIONS`, because I haven't got enough time to move the FFmpeg's arguments in. But this program is suppose to work well when they are added. And currently this program support Option and OptionGroup quite well, because I directly ported the `split_commandline` function of FFmpeg to Rust(with little adjustment to make it more "Rusty" though).

`AVOptions` is not supported.

You can test by using `cargo test`. And you can use `RUST_LOG=debug cargo run -- -i input.mp4 ouput.mkv` to check program outputs (I changed the `av_log`s in FFmpeg's source code to `log::{debug, error}'). So you will see the exact same debug message when specific parameters are given.

It's under the original license of FFmpeg.