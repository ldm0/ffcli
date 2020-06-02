use bitflags::bitflags;
use libc::c_void;
use std::{default, fmt, marker};

bitflags! {
    #[derive(Default)]
    pub struct OptionFlag: u64 {
        const NONE          = 0x0000;
        const HAS_ARG       = 0x0001;
        const OPT_BOOL      = 0x0002;
        const OPT_EXPERT    = 0x0004;
        const OPT_STRING    = 0x0008;
        const OPT_VIDEO     = 0x0010;
        const OPT_AUDIO     = 0x0020;
        const OPT_INT       = 0x0080;
        const OPT_FLOAT     = 0x0100;
        const OPT_SUBTITLE  = 0x0200;
        const OPT_INT64     = 0x0400;
        const OPT_EXIT      = 0x0800;
        const OPT_DATA      = 0x1000;
        const OPT_PERFILE   = 0x2000;
        const OPT_OFFSET    = 0x4000;
        const OPT_SPEC      = 0x8000;
        const OPT_TIME      = 0x10000;
        const OPT_DOUBLE    = 0x20000;
        const OPT_INPUT     = 0x40000;
        const OPT_OUTPUT    = 0x80000;
    }
}

pub union OptionOperation {
    pub dst_ptr: *mut c_void,
    pub func_arg: fn(*mut c_void, &str, &str) -> i64,
    pub off: usize,
}

impl fmt::Debug for OptionOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("(Union)OptionOperation")
            .field("val", unsafe { &self.off })
            .finish()
    }
}

impl default::Default for OptionOperation {
    fn default() -> Self {
        OptionOperation { off: 0 }
    }
}

#[derive(Debug, Default)]
pub struct OptionDef<'a> {
    pub name: &'a str,
    pub help: &'a str,
    pub argname: Option<&'a str>,
    pub flags: OptionFlag,
    pub u: OptionOperation,
}

/// Though OptionOperation contains pointer, we still need it to impl Send and
/// Sync, we can ensure its safety.
unsafe impl<'a> marker::Send for OptionDef<'a> {}

/// Though OptionOperation contains pointer, we still need it to impl Send and
/// Sync, we can ensure its safety.
unsafe impl<'a> marker::Sync for OptionDef<'a> {}

/// Currently move the flags out of the struct.
#[derive(Debug, Default)]
pub struct OptionGroupDef<'global> {
    pub name: &'global str,
    pub sep: Option<&'global str>,
    pub flags: OptionFlag,
}

/// Original name is `Option` in FFmpeg, but it's a wide-use type in Rust.
/// So I rename it to `OptionKV`.
#[derive(Debug, Clone)]
pub struct OptionKV<'global> {
    pub opt: &'global OptionDef<'global>,
    pub key: String,
    pub val: String,
}

// TODO maybe split the lifetime here
#[derive(Debug, Clone)]
pub struct OptionGroup<'global> {
    pub group_def: &'global OptionGroupDef<'global>,
    pub arg: String,
    pub opts: Vec<OptionKV<'global>>,
    /* Ignore them currently
    AVDictionary *codec_opts;
    AVDictionary *format_opts;
    AVDictionary *resample_opts;
    AVDictionary *sws_dict;
    AVDictionary *swr_opts;
    */
}

impl<'global> OptionGroup<'global> {
    pub fn new_global() -> Self {
        static GLOBAL_GROUP: OptionGroupDef = OptionGroupDef {
            name: "global",
            sep: None,
            flags: OptionFlag::NONE,
        };
        OptionGroup {
            group_def: &GLOBAL_GROUP,
            arg: String::new(),
            opts: vec![],
        }
    }

    /// This function is specially used for cur_group before it's
    /// refactored into tuple.
    pub fn new_anonymous() -> Self {
        static NEVER_USE_GROUP: OptionGroupDef = OptionGroupDef {
            name: "never_used",
            sep: None,
            flags: OptionFlag::NONE,
        };
        OptionGroup {
            group_def: &NEVER_USE_GROUP,
            arg: String::new(),
            opts: vec![],
        }
    }
}

/// A list of option groups that all have the same group type
/// (e.g. input files or output files)
#[derive(Debug)]
pub struct OptionGroupList<'global> {
    pub group_def: &'global OptionGroupDef<'global>,
    pub groups: Vec<OptionGroup<'global>>,
}

#[derive(Debug)]
pub struct OptionParseContext<'global> {
    /// Global options
    pub global_opts: OptionGroup<'global>,
    /// Options that can find a OptionGroupDef
    pub groups: Vec<OptionGroupList<'global>>,
    /// Parsing state
    /// Attention: The group_def in the cur_group has never been used, so we just
    /// use create a placeholder. More attractive option is change the cur_group
    /// from OptionGroup to tuple (arg: String, opts: Vec<OptionKV>).
    pub cur_group: OptionGroup<'global>,
}

pub union SpecifierOptValue {
    pub str: *mut u8,
    pub i: isize,
    pub i64: i64,
    pub ui64: u64,
    pub f: f32,
    pub dbl: f64,
}

impl default::Default for SpecifierOptValue {
    fn default() -> Self {
        SpecifierOptValue { i: 0 }
    }
}

pub struct SpecifierOpt {
    pub specifier: String,
    pub u: SpecifierOptValue,
}

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

#[cfg(test)]
mod types_tests {
    use super::*;

    #[test]
    fn fmt_debug_option_operation_default() {
        let optop: OptionOperation = Default::default();
        assert_eq!(format!("{:?}", optop), "(Union)OptionOperation { val: 0 }");
    }

    #[test]
    fn fmt_debug_option_operation() {
        let optop: OptionOperation = OptionOperation { off: 123_456 };
        assert_eq!(
            format!("{:?}", optop),
            "(Union)OptionOperation { val: 123456 }"
        );
    }
}
