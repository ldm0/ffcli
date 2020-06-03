//! This file corresponds to ffmpeg.\[ch\]
use std::env;

use crate::{
    cmdutils::{OptionGroup, SpecifierOpt},
    ffmpeg_opt,
};

use ffmpeg_opt::ffmpeg_parse_options;

pub struct StreamMap {
    pub disabled: isize,
    pub file_index: isize,
    pub stream_index: isize,
    pub sync_file_index: isize,
    pub sync_stream_index: isize,
    pub linklabel: String,
}

pub struct AudioChannelMap {
    // input
    pub file_idx: isize,
    pub stream_idx: isize,
    pub channel_idx: isize,
    // output
    pub ofile_idx: isize,
    pub ostream_idx: isize,
}

pub struct OptionsContext<'a, 'global> {
    pub g: OptionGroup<'global>,

    // input/output options
    pub start_time: i64,
    pub start_time_eof: i64,
    pub seek_timestamp: isize,
    pub format: &'a str,

    pub codec_names: Vec<SpecifierOpt>,
    pub audio_channels: Vec<SpecifierOpt>,
    pub audio_sample_rate: Vec<SpecifierOpt>,
    pub frame_rates: Vec<SpecifierOpt>,
    pub frame_sizes: Vec<SpecifierOpt>,
    pub frame_pix_fmts: Vec<SpecifierOpt>,

    // input options
    pub input_ts_offset: i64,
    pub loops: isize,
    pub rate_emu: isize,
    pub accurate_seek: isize,
    pub thread_queue_size: isize,

    pub ts_scale: Vec<SpecifierOpt>,
    pub dump_attachment: Vec<SpecifierOpt>,
    pub hwaccels: Vec<SpecifierOpt>,
    pub hwaccel_devices: Vec<SpecifierOpt>,
    pub hwaccel_output_formats: Vec<SpecifierOpt>,
    pub autorotate: Vec<SpecifierOpt>,

    // output options
    pub stream_maps: Vec<StreamMap>,
    // ATTENTION here does the nb_audio* is the length of the audio* array?
    // I'm not sure. Currently I assume they are the same. If not we need to a a integer here.
    // AudioChannelMap *audio_channel_maps; /* one info entry per -map_channel */
    // int           nb_audio_channel_maps; /* number of (valid) -map_channel settings */
    pub audio_channel_maps: Vec<AudioChannelMap>,
    pub metadata_global_manual: isize,
    pub metadata_streams_manual: isize,
    pub metadata_chapters_manual: isize,
    pub attachments: Vec<String>,

    pub chapters_input_file: isize,

    pub recording_time: i64,
    pub stop_time: i64,
    pub limit_filesize: u64,
    pub mux_preload: f32,
    pub mux_max_delay: f32,
    pub shortest: isize,
    pub bitexact: isize,

    pub video_disable: isize,
    pub audio_disable: isize,
    pub subtitle_disable: isize,
    pub data_disable: isize,

    // indexed by output file stream index
    pub streamid_map: Vec<isize>,

    pub metadata: Vec<SpecifierOpt>,
    pub max_frames: Vec<SpecifierOpt>,
    pub bitstream_filters: Vec<SpecifierOpt>,
    pub codec_tags: Vec<SpecifierOpt>,
    pub sample_fmts: Vec<SpecifierOpt>,
    pub qscale: Vec<SpecifierOpt>,
    pub forced_key_frames: Vec<SpecifierOpt>,
    pub force_fps: Vec<SpecifierOpt>,
    pub frame_aspect_ratios: Vec<SpecifierOpt>,
    pub rc_overrides: Vec<SpecifierOpt>,
    pub intra_matrices: Vec<SpecifierOpt>,
    pub inter_matrices: Vec<SpecifierOpt>,
    pub chroma_intra_matrices: Vec<SpecifierOpt>,
    pub top_field_first: Vec<SpecifierOpt>,
    pub metadata_map: Vec<SpecifierOpt>,
    pub presets: Vec<SpecifierOpt>,
    pub copy_initial_nonkeyframes: Vec<SpecifierOpt>,
    pub copy_prior_start: Vec<SpecifierOpt>,
    pub filters: Vec<SpecifierOpt>,
    pub filter_scripts: Vec<SpecifierOpt>,
    pub reinit_filters: Vec<SpecifierOpt>,
    pub fix_sub_duration: Vec<SpecifierOpt>,
    pub canvas_sizes: Vec<SpecifierOpt>,
    pub pass: Vec<SpecifierOpt>,
    pub passlogfiles: Vec<SpecifierOpt>,
    pub max_muxing_queue_size: Vec<SpecifierOpt>,
    pub guess_layout_max: Vec<SpecifierOpt>,
    pub apad: Vec<SpecifierOpt>,
    pub discard: Vec<SpecifierOpt>,
    pub disposition: Vec<SpecifierOpt>,
    pub program: Vec<SpecifierOpt>,
    pub time_bases: Vec<SpecifierOpt>,
    pub enc_time_bases: Vec<SpecifierOpt>,
}

pub fn ffmpeg() {
    // TODO: May need to change to Vec<u8> for non-UTF8 args.
    let args: Vec<String> = env::args().collect();

    ffmpeg_parse_options(&args);
}
