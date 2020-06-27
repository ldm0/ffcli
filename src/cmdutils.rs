use bitflags::bitflags;
use libc::c_void;
use log::{debug, error, info};
use once_cell::sync::Lazy;
use rusty_ffmpeg::{
    avutil::{avutils::*, error::*},
    ffi,
};

use std::{
    cmp,
    collections::HashMap,
    default,
    ffi::{CStr, CString},
    fmt, marker, mem, ptr, slice,
    sync::Mutex,
};

use crate::ffmpeg::OptionsContext;

enum OptGroup {
    GroupOutfile = 0,
    GroupInfile = 1,
}

// TODO implement all error number later, might in a separate file
// TODO change this to FFERRTAG later.
const AVERROR_OPTION_NOT_FOUND: isize = 3;

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

// Consider changing them to AVDictionary if needed.
static FORMAT_OPTS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static CODEC_OPTS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static SWS_DICT: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static SWR_OPTS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static RESAMPLE_OPTS: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

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
    pub codec_opts: *mut ffi::AVDictionary,
    pub format_opts: *mut ffi::AVDictionary,
    pub resample_opts: *mut ffi::AVDictionary,
    pub sws_dict: *mut ffi::AVDictionary,
    pub swr_opts: *mut ffi::AVDictionary,
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
            codec_opts: ptr::null_mut(),
            format_opts: ptr::null_mut(),
            resample_opts: ptr::null_mut(),
            sws_dict: ptr::null_mut(),
            swr_opts: ptr::null_mut(),
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
            codec_opts: ptr::null_mut(),
            format_opts: ptr::null_mut(),
            resample_opts: ptr::null_mut(),
            sws_dict: ptr::null_mut(),
            swr_opts: ptr::null_mut(),
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
    /// use create a placeholder. More attractive option is changing the
    /// cur_group from OptionGroup to tuple (arg: String, opts: Vec<OptionKV>).
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

impl fmt::Debug for SpecifierOptValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("(Union)SpecifierOptValue")
            .field("val", unsafe { &self.i })
            .finish()
    }
}

impl default::Default for SpecifierOptValue {
    fn default() -> Self {
        SpecifierOptValue { i: 0 }
    }
}

#[derive(Debug, Default)]
pub struct SpecifierOpt {
    pub specifier: String,
    pub u: SpecifierOptValue,
}

/// This function accepts moved Option value with the OptionsContext it references to unchanged.
pub fn parse_optgroup<'ctxt>(
    mut optctx: Option<&mut OptionsContext>,
    g: &OptionGroup,
) -> Result<(), ()> {
    debug!(
        "Parsing a group of options: {} {}.",
        g.group_def.name, g.arg
    );
    g.opts
        .iter()
        .map(|o| {
            if !g.group_def.flags.is_empty() && !g.group_def.flags.intersects(o.opt.flags) {
                error!(
                    "Option {} ({}) cannot be applied to \
                   {} {} -- you are trying to apply an input option to an \
                   output file or vice versa. Move this option before the \
                   file it belongs to.",
                    o.key, o.opt.help, g.group_def.name, g.arg
                );
                Err(())
            } else {
                debug!(
                    "Applying option {} ({}) with argument {}.",
                    o.key, o.opt.help, o.val
                );
                write_option(&mut optctx, o.opt, &o.key, &o.val)
            }
        })
        .collect::<Result<Vec<_>, ()>>()?;
    debug!("Successfully parsed a group of options.");
    Ok(())
}

/// `context` is the `opt`, `num_str` is usually the `arg`
pub fn parse_number(
    context: &str,
    numstr: &str,
    num_type: OptionFlag,
    min: f64,
    max: f64,
) -> Result<f64, String> {
    let numstr_ptr = CString::new(numstr).unwrap().as_ptr();
    let mut tail: *mut libc::c_char = ptr::null_mut();
    let d = unsafe { ffi::av_strtod(numstr_ptr, &mut tail) };
    let error = if tail.is_null() {
        format!("Expected number for {} but found: {}", context, numstr)
    } else {
        if d < min || d > max {
            format!(
                "The value for {} was {} which is not within {} - {}",
                context, numstr, min, max
            )
        } else if num_type == OptionFlag::OPT_INT64 && d as i64 as f64 != d {
            format!("Expected int64 for {} but found {}", context, numstr)
        } else if num_type == OptionFlag::OPT_INT && d as isize as f64 != d {
            format!("Expected int for {} but found {}", context, numstr)
        } else {
            return Ok(d);
        }
    };
    Err(error)
}

fn parse_time(context: &str, timestr: &str, is_duration: bool) -> Result<i64, String> {
    let mut us = 0;
    let timestr_ptr = CString::new(timestr).unwrap().as_ptr();
    if unsafe { ffi::av_parse_time(&mut us, timestr_ptr, if is_duration { 1 } else { 0 }) } > 0 {
        Err(format!(
            "Invalid {} specification for {}: {}",
            if is_duration { "duration" } else { "date" },
            context,
            timestr
        ))
    } else {
        Ok(us)
    }
}

/// If failed, panic with some description.
/// TODO: change this function to return corresponding Result later
fn write_option(
    optctx: &mut Option<&mut OptionsContext>,
    po: &OptionDef,
    opt: &str,
    arg: &str,
) -> Result<(), ()> {
    let dst: *mut c_void = if po
        .flags
        .intersects(OptionFlag::OPT_OFFSET | OptionFlag::OPT_SPEC)
    {
        if let &mut Some(ref mut optctx) = optctx {
            *optctx as *mut _ as *mut c_void
        } else {
            panic!("some option contains OPT_OFFSET or OPT_SPEC but in global_opts")
        }
    } else {
        unsafe { po.u.dst_ptr }
    };

    if po.flags.contains(OptionFlag::OPT_SPEC) {
        let so = dst as *mut Vec<SpecifierOpt>;
        let so = unsafe { so.as_mut() }.unwrap();
        let s = opt.find(':').map_or("", |i| &opt[i + 1..]);
        so.push(SpecifierOpt {
            specifier: s.to_owned(),
            u: Default::default(),
        });
    }

    if po.flags.contains(OptionFlag::OPT_STRING) {
        let dst = dst as *mut String;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = arg.to_owned();
    } else if po
        .flags
        .intersects(OptionFlag::OPT_STRING | OptionFlag::OPT_INT)
    {
        let dst = dst as *mut isize;
        let dst = unsafe { dst.as_mut() }.unwrap();
        // IMPROVEMENT FFmpeg uses i32::{MIN, MAX} here but it's int though many
        // c compiler still treat int as 32bit, but I think for Rust age, we
        // need to change it.
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            isize::MIN as f64,
            isize::MAX as f64,
        )
        .unwrap() as isize;
    } else if po.flags.contains(OptionFlag::OPT_INT64) {
        let dst = dst as *mut i64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap() as i64;
    } else if po.flags.contains(OptionFlag::OPT_TIME) {
        let dst = dst as *mut i64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_time(opt, arg, true).unwrap();
    } else if po.flags.contains(OptionFlag::OPT_FLOAT) {
        let dst = dst as *mut f32;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap() as f32;
    } else if po.flags.contains(OptionFlag::OPT_DOUBLE) {
        let dst = dst as *mut f64;
        let dst = unsafe { dst.as_mut() }.unwrap();
        *dst = parse_number(
            opt,
            arg,
            OptionFlag::OPT_INT64,
            i64::MIN as f64,
            i64::MAX as f64,
        )
        .unwrap();
    } else if unsafe { po.u.off } != 0 {
        let optctx = if let &mut Some(ref mut optctx) = optctx {
            *optctx as *mut _ as *mut c_void
        } else {
            panic!("Option contains function pointer but in global_opts");
        };
        let func = unsafe { po.u.func_arg };
        let ret = func(optctx, opt, arg);
        // TODO av_err2str() still haven't been implemented
        if ret < 0 {
            error!(
                "Failed to set value '{}' for option '{}': {}",
                arg, opt, "av_err2str()"
            );
            return Err(());
        }
    }
    if po.flags.contains(OptionFlag::OPT_EXIT) {
        panic!("exit as required");
    }
    Ok(())
}

// TODO the Err in returned Result need to be a ERROR enum
pub fn split_commandline<'ctxt, 'global>(
    octx: &'ctxt mut OptionParseContext<'global>,
    args: &[String],
    options: &'global [OptionDef],
    groups: &'global [OptionGroupDef],
) -> Result<(), ()> {
    let argv = args;
    let argc = argv.len();

    // No app arguments preparation, and the init_parse_context is moved outside.

    debug!("Splitting the commandline.");

    let mut optindex = 1;
    let mut dashdash = None;

    // consider using `Iterator::nth()` to replace the `while` with `for`
    while optindex < argc {
        let opt = &argv[optindex];
        optindex += 1;

        debug!("Reading option '{}' ...", opt);

        if opt == "--" {
            dashdash = Some(optindex);
            continue;
        }

        // unnamed group separators, e.g. output filename
        if !opt.starts_with('-') || opt.len() <= 1 || dashdash == Some(optindex - 1) {
            // IMPROVEMENT original FFmpeg uses 0 rather than enum value here.
            finish_group(octx, OptGroup::GroupOutfile as usize, opt);
            debug!(
                " matched as {}.",
                groups[OptGroup::GroupOutfile as usize].name
            );
            continue;
        }

        // Jump over prefix `-`
        let opt = &opt[1..];

        // Named group separators, e.g. -i
        if let Some(group_idx) = match_group_separator(groups, opt) {
            let arg = match argv.get(optindex) {
                Some(arg) => arg,
                None => return Err(()),
            };
            optindex += 1;

            finish_group(octx, group_idx, arg);
            debug!(
                " matched as {} with argument '{}'.",
                groups[group_idx].name, arg
            );
            continue;
        }

        // Normal options
        if let Some(po) = find_option(options, opt) {
            let arg = if po.flags.intersects(OptionFlag::OPT_EXIT) {
                // Optional argument, e.g. -h

                // Yes, we cannot use unwrap_or() here because a coercion needed.
                let arg = match argv.get(optindex) {
                    Some(x) => x,
                    None => "",
                };
                optindex += 1;
                arg
            } else if po.flags.intersects(OptionFlag::HAS_ARG) {
                let arg = match argv.get(optindex) {
                    Some(x) => x,
                    None => return Err(()),
                };
                optindex += 1;
                arg
            } else {
                "1"
            };
            add_opt(octx, po, opt, arg);
            debug!(
                " matched as option '{}' ({}) with argument '{:?}'.",
                po.name, po.help, arg
            );
            continue;
        }

        // AVOptions
        if let Some(arg) = argv.get(optindex) {
            // Hint: `rust_analyzer` failed to parse following code
            // Process common options and process AVOption by the way(the
            // function name is not that self-explaining), **where some global
            // option directory is fulfilled**(this is extremely weird for me to
            // understand).
            match opt_default(ptr::null_mut(), opt, arg) {
                0.. => {
                    debug!(" matched as AVOption '{}' with argument '{}'.", opt, arg);
                    optindex += 1;
                    continue;
                }
                AVERROR_OPTION_NOT_FOUND => {
                    debug!("Error parsing option '{}' with argument '{}'.\n", opt, arg);
                    return Err(());
                }
                _ => {}
            }
        }

        // boolean -nofoo options
        if opt.starts_with("no") {
            if let Some(po) = find_option(options, &opt[2..]) {
                if po.flags.contains(OptionFlag::OPT_BOOL) {
                    debug!(
                        " matched as option '{}' ({}) with argument 0.",
                        po.name, po.help
                    );
                    continue;
                }
            }
        }

        error!("Unrecognized option '{}'.", opt);
        return Err(());
    }

    if !octx.cur_group.opts.is_empty()
        || !CODEC_OPTS.lock().unwrap().is_empty()
        || !FORMAT_OPTS.lock().unwrap().is_empty()
        || !RESAMPLE_OPTS.lock().unwrap().is_empty()
    {
        debug!("Trailing option(s) found in the command: may be ignored.");
    }

    debug!("Finished splitting the commandline.");
    Ok(())
}

fn opt_default(_: *mut c_void, opt: &str, arg: &str) -> isize {
    if opt == "debug" || opt == "fdebug" {
        // TODO implement equivalent function of av_log_set_level()
        info!("debug is currently not implemented, debug is the default");
    }
    let opt_stripped = CString::new(opt.split(':').next().unwrap()).unwrap();
    let opt_head = opt.chars().next();
    // This is unicode-safe because it's only used when first char is ascii.
    let opt_nohead = opt.get(1..).map(|x| CString::new(x).unwrap());

    let opt_c = CString::new(opt).unwrap();
    let arg_c = CString::new(arg).unwrap();

    let (opt_ptr, arg_ptr) = (opt_c.as_ptr(), arg_c.as_ptr());

    let mut cc = unsafe { ffi::avcodec_get_class() };
    let mut fc = unsafe { ffi::avformat_get_class() };
    /* Currently not supported, they seems to be used less often.
    let sc = sws_get_class();
    let swr_class = swr_get_class();
    */

    let mut consumed = false;
    if opt_find(
        &mut cc as *mut _ as *mut c_void,
        opt_stripped.as_ptr(),
        ptr::null(),
        0,
        ffi::AV_OPT_SEARCH_CHILDREN | ffi::AV_OPT_SEARCH_FAKE_OBJ,
    ) || ((opt_head == Some('v') || opt_head == Some('a') || opt_head == Some('s'))
        && opt_find(
            &mut cc as *mut _ as *mut c_void,
            opt_nohead.unwrap().as_ptr(),
            ptr::null(),
            0,
            ffi::AV_OPT_SEARCH_FAKE_OBJ,
        ))
    {
        CODEC_OPTS.lock().unwrap().insert(opt.into(), arg.into());
        consumed = true;
    }
    if opt_find(
        &mut fc as *mut _ as *mut c_void,
        opt_ptr,
        ptr::null(),
        0,
        ffi::AV_OPT_SEARCH_CHILDREN | ffi::AV_OPT_SEARCH_FAKE_OBJ,
    ) {
        FORMAT_OPTS.lock().unwrap().insert(opt.into(), arg.into());
        consumed = true;
    }

    // TODO: init things about SWRESAMPLE SWSCALE

    if consumed {
        0
    } else {
        AVERROR_OPTION_NOT_FOUND
    }
}

/// Whether a valid option is found.
fn opt_find(
    obj: *mut c_void,
    name: *const libc::c_char,
    unit: *const libc::c_char,
    opt_flags: u32,
    search_flags: u32,
) -> bool {
    let o = unsafe { ffi::av_opt_find(obj, name, unit, opt_flags as i32, search_flags as i32) };
    if o.is_null() {
        false
    } else if unsafe { o.as_ref() }.unwrap().flags == 0 {
        false
    } else {
        true
    }
}

fn match_group_separator(groups: &[OptionGroupDef], opt: &str) -> Option<usize> {
    groups
        .iter()
        .enumerate()
        .find_map(|(i, optdef)| Some(i).filter(|_| optdef.sep == Some(opt)))
}

/// Finish parsing an option group. Move current parsing group into specific group list
/// # Parameters
/// `group_idx`     which group definition should this group belong to
/// `arg`           argument of the group delimiting option
fn finish_group(octx: &mut OptionParseContext, group_idx: usize, arg: &str) {
    let mut new_group = octx.cur_group.clone();
    new_group.arg = arg.to_owned();
    new_group.group_def = octx.groups[group_idx].group_def;

    octx.groups[group_idx].groups.push(new_group);
    octx.cur_group = OptionGroup::new_anonymous();
    init_opts();
}

fn init_opts() {
    SWS_DICT
        .lock()
        .unwrap()
        .insert("flags".into(), "bicubic".into());
}

fn find_option<'global>(
    options: &'global [OptionDef<'global>],
    name: &str,
) -> Option<&'global OptionDef<'global>> {
    // TODAY can theses two lines be simplified?
    let mut splits = name.split(':');
    let name = splits.next()?;
    options.iter().find(|&option_def| option_def.name == name)
}

/// Add an option instance to currently parsed group.
fn add_opt<'ctxt, 'global>(
    octx: &'ctxt mut OptionParseContext<'global>,
    opt: &'global OptionDef<'global>,
    key: &str,
    val: &str,
) {
    let global = !opt
        .flags
        .intersects(OptionFlag::OPT_PERFILE | OptionFlag::OPT_SPEC | OptionFlag::OPT_OFFSET);
    let g = if global {
        // Here we can ensure that global_opts's flags doesn't contains either OPT_SPEC or OPT_OFFSET
        &mut octx.global_opts
    } else {
        &mut octx.cur_group
    };
    g.opts.push(OptionKV {
        opt: opt,
        key: key.to_owned(),
        val: val.to_owned(),
    })
}

pub fn print_error(filename: &str, err: i32) {
    let mut errbuf = [0 as libc::c_char; 128];
    let errbuf_ptr = unsafe {
        if ffi::av_strerror(err, &mut errbuf as *mut _ as *mut i8, 128) < 0 {
            CString::from_raw(ffi::strerror(AVUNERROR(err)))
        } else {
            CString::from_raw(&mut errbuf as *mut _ as *mut i8)
        }
    };

    error!("{}: {}", filename, errbuf_ptr.to_str().unwrap());
}

unsafe fn check_stream_specifier(
    s: *mut ffi::AVFormatContext,
    st: *mut ffi::AVStream,
    spec: *const libc::c_char,
) -> libc::c_int {
    let ret = ffi::avformat_match_stream_specifier(s, st, spec);
    let spec = CStr::from_ptr(spec);
    if ret < 0 {
        error!("Invalid stream specifier: {}.", spec.to_string_lossy());
    }
    ret
}

unsafe fn filter_codec_opts(
    opts_ptr: *mut ffi::AVDictionary,
    codec_id: ffi::AVCodecID,
    s_ptr: *mut ffi::AVFormatContext,
    st_ptr: *mut ffi::AVStream,
    codec_ptr: *mut ffi::AVCodec,
) -> *mut ffi::AVDictionary {
    let s = s_ptr.as_ref().unwrap();
    let st = st_ptr.as_ref().unwrap();
    let mut cc = ffi::avcodec_get_class();

    let mut flags = if !s.oformat.is_null() {
        ffi::AV_OPT_FLAG_ENCODING_PARAM
    } else {
        ffi::AV_OPT_FLAG_DECODING_PARAM
    };

    let codec_ptr = if codec_ptr.is_null() {
        if !s.oformat.is_null() {
            ffi::avcodec_find_encoder(codec_id)
        } else {
            ffi::avcodec_find_decoder(codec_id)
        }
    } else {
        codec_ptr
    };

    let prefix = match st.codecpar.as_ref().unwrap().codec_type {
        ffi::AVMediaType_AVMEDIA_TYPE_VIDEO => {
            flags |= ffi::AV_OPT_FLAG_VIDEO_PARAM;
            b'v'
        }
        ffi::AVMediaType_AVMEDIA_TYPE_AUDIO => {
            flags |= ffi::AV_OPT_FLAG_AUDIO_PARAM;
            b'a'
        }
        ffi::AVMediaType_AVMEDIA_TYPE_SUBTITLE => {
            flags |= ffi::AV_OPT_FLAG_SUBTITLE_PARAM;
            b's'
        }
        _ => 0 as u8,
    };

    let mut ret = ptr::null_mut();
    let mut t_ptr = ptr::null_mut();

    let empty = CString::new("").unwrap();

    loop {
        t_ptr = ffi::av_dict_get(
            opts_ptr,
            empty.as_ptr(),
            t_ptr,
            ffi::AV_DICT_IGNORE_SUFFIX as i32,
        );
        let t = match t_ptr.as_ref() {
            Some(t) => t,
            None => break,
        };

        let p = libc::strchr(t.key, b':' as _);

        // check stream specification in opt name
        if !p.is_null() {
            match check_stream_specifier(s_ptr, st_ptr, p.offset(1)) {
                1 => {
                    *p = 0;
                    break;
                }
                0 => {
                    continue;
                }
                _ => panic!(),
            }
        }

        if !ffi::av_opt_find(
            &mut cc as *mut _ as *mut c_void,
            t.key,
            ptr::null(),
            flags as _,
            ffi::AV_OPT_SEARCH_FAKE_OBJ as _,
        )
        .is_null()
            || codec_ptr.is_null()
            || (!codec_ptr.as_ref().unwrap().priv_class.is_null()
                && !ffi::av_opt_find(
                    &mut codec_ptr.as_mut().unwrap().priv_class as *mut _ as *mut c_void,
                    t.key,
                    ptr::null(),
                    flags as _,
                    ffi::AV_OPT_SEARCH_FAKE_OBJ as _,
                )
                .is_null())
        {
            ffi::av_dict_set(&mut ret as *mut _, t.key, t.value, 0);
        } else if *t.key == prefix as i8
            && !ffi::av_opt_find(
                &mut cc as *mut _ as *mut c_void,
                t.key.offset(1),
                ptr::null(),
                flags as i32,
                ffi::AV_OPT_SEARCH_FAKE_OBJ as i32,
            )
            .is_null()
        {
            ffi::av_dict_set(&mut ret as *mut _, t.key.offset(1), t.value, 0);
        }
        if !p.is_null() {
            *p = b':' as _;
        }
    }

    ret
}

pub unsafe fn setup_find_stream_info_opts(
    s: *mut ffi::AVFormatContext,
    codec_opts: *mut ffi::AVDictionary,
) -> *mut *mut ffi::AVDictionary {
    let s_ptr = s;
    let s = s_ptr.as_ref().unwrap();
    if s.nb_streams == 0 {
        return ptr::null_mut();
    }
    let opts_ptr = ffi::av_mallocz_array(
        s.nb_streams as u64,
        mem::size_of::<*mut ffi::AVDictionary>() as u64,
    ) as *mut *mut ffi::AVDictionary;

    if opts_ptr.is_null() {
        error!("Could not alloc memory for stream options.");
        ptr::null_mut()
    } else {
        let opts = slice::from_raw_parts_mut(opts_ptr, s.nb_streams as usize);
        let s_streams = slice::from_raw_parts_mut(s.streams, s.nb_streams as usize);
        for (opt, s_stream) in opts.iter_mut().zip(s_streams.iter()) {
            let codec_id = s_stream
                .as_ref()
                .unwrap()
                .codecpar
                .as_ref()
                .unwrap()
                .codec_id;
            *opt = filter_codec_opts(codec_opts, codec_id, s_ptr, *s_stream, ptr::null_mut());
        }
        opts_ptr
    }
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
