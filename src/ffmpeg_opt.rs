use libc::c_void;
use log::{debug, error, info};
use rusty_ffmpeg::{
    avutil::{avutils::*, error::*},
    ffi,
};
use std::{
    ffi::{CStr, CString},
    ptr, slice,
};

use crate::{
    cmdutils::{
        // need to remove the directly imported functions
        self,
        parse_optgroup,
        print_error,
        split_commandline,
        OptionDef,
        OptionFlag,
        OptionGroup,
        OptionGroupDef,
        OptionGroupList,
        OptionKV,
        OptionOperation,
        OptionParseContext,
        SpecifierOpt,
        SpecifierOptValue,
    },
    ffmpeg::{self, OptionsContext, INT_CB},
    options::*,
};

enum OptGroup {
    GroupOutFile = 0,
    GroupInFile = 1,
}

fn open_files(
    l: &mut OptionGroupList,
    inout: &str,
    open_file: fn(&mut OptionsContext, &str) -> isize,
) -> Result<(), ()> {
    for g in l.groups.iter_mut() {
        let g_arg = g.arg.clone();
        // This is a workaround for g's immutable borrow while g is mutably
        // borrowed, consider that g's properties use by this function are not
        // modified in the parse_optgroup.
        let g_ = g.clone();
        let mut o = OptionsContext::new(g);
        if let Err(ret) = parse_optgroup(Some(&mut o), &g_) {
            error!("Error parsing options for {} file {}.", inout, g.arg);
            return Err(ret);
        }
        debug!("Opening an {} file: {}.", inout, g_arg);
        let ret = open_file(&mut o, &g_arg);
        if ret < 0 {
            error!("Error opening {} file {}.\n", inout, g.arg);
            return Err(());
        }
        debug!("Successfully opened the file.");
    }
    Ok(())
}

fn open_input_file(o: &mut OptionsContext, filename: &str) -> isize {
    if o.stop_time != i64::MAX && o.recording_time != i64::MAX {
        o.stop_time = i64::MAX;
        error!("-t and -to cannot be used together; using -t.");
    }
    if o.stop_time != i64::MAX && o.recording_time == i64::MAX {
        let start_time = if o.start_time == AV_NOPTS_VALUE {
            0
        } else {
            o.start_time
        };
        if o.stop_time <= start_time {
            panic!("-to value smaller than -ss; aborting.");
        } else {
            o.recording_time = o.stop_time - start_time;
        }
    }

    let file_iformat_ptr = if !o.format.is_empty() {
        let o_format = CString::new(&o.format as &str).unwrap();
        let file_iformat = unsafe { ffi::av_find_input_format(o_format.as_ptr()) };
        if file_iformat.is_null() {
            panic!("Unknown input format: '{}'", o.format);
        }
        file_iformat
    } else {
        ptr::null_mut()
    };
    let mut file_iformat = unsafe { file_iformat_ptr.as_mut() };

    let filename = match filename {
        "-" => "pipe:",
        _ => filename,
    };

    let file_stdin = if filename.starts_with("pipe:") && filename != "/dev/stdin" {
        1
    } else {
        0
    };
    unsafe {
        stdin_interaction = stdin_interaction & file_stdin;
    }

    // get default parameters from command line
    let ic = match unsafe { ffi::avformat_alloc_context().as_mut() } {
        Some(x) => x,
        None => {
            print_error(filename, AVERROR(ffi::ENOMEM as i32));
            panic!();
        }
    };

    if !o.audio_sample_rate.is_empty() {
        let tmp = CString::new("sameple_rate").unwrap();
        unsafe {
            ffi::av_dict_set_int(
                &mut o.g.format_opts as *mut _,
                tmp.as_ptr(),
                o.audio_sample_rate[o.audio_sample_rate.len() - 1].u.i as i64,
                0,
            );
        }
    }

    if !o.audio_channels.is_empty() {
        // Because we set audio_channels based on both the "ac" and
        // "channel_layout" options, we need to check that the specified
        // demuxer actually has the "channels" option before setting it */
        if let Some(file_iformat) = &mut file_iformat {
            let channels = CString::new("channels").unwrap();
            if !unsafe {
                ffi::av_opt_find(
                    &mut file_iformat.priv_class as *mut _ as *mut c_void,
                    channels.as_ptr(),
                    ptr::null(),
                    0,
                    ffi::AV_OPT_SEARCH_FAKE_OBJ as i32,
                )
            }
            .is_null()
            {
                unsafe {
                    ffi::av_dict_set_int(
                        &mut o.g.format_opts as *mut _,
                        channels.as_ptr(),
                        o.audio_channels[o.audio_channels.len() - 1].u.i as i64,
                        0,
                    );
                }
            }
        }
    }

    if !o.frame_rates.is_empty() {
        // set the format-level framerate option;
        // this is important for video grabbers, e.g. x11
        if let Some(file_iformat) = &mut file_iformat {
            let framerate = CString::new("framerate").unwrap();
            if !unsafe {
                ffi::av_opt_find(
                    &mut file_iformat.priv_class as *mut _ as *mut c_void,
                    framerate.as_ptr(),
                    ptr::null(),
                    0,
                    ffi::AV_OPT_SEARCH_FAKE_OBJ as i32,
                )
            }
            .is_null()
            {
                unsafe {
                    ffi::av_dict_set(
                        &mut o.g.format_opts as *mut _,
                        framerate.as_ptr(),
                        o.frame_rates[o.frame_rates.len() - 1].u.str as *const _ as *const i8,
                        0,
                    );
                }
            }
        }
    }
    if !o.frame_sizes.is_empty() {
        let video_size = CString::new("video_size").unwrap();
        unsafe {
            ffi::av_dict_set(
                &mut o.g.format_opts as *mut _,
                video_size.as_ptr(),
                o.frame_sizes[o.frame_sizes.len() - 1].u.str as *const _ as *const i8,
                0,
            );
        }
    }

    if !o.frame_pix_fmts.is_empty() {
        let pixel_format = CString::new("pixel_format").unwrap();
        unsafe {
            ffi::av_dict_set(
                &mut o.g.format_opts as *mut _,
                pixel_format.as_ptr(),
                o.frame_pix_fmts[o.frame_pix_fmts.len() - 1].u.str as *const _ as *const i8,
                0,
            );
        }
    }

    let mut video_codec_name = ptr::null_mut();
    let mut audio_codec_name = ptr::null_mut();
    let mut subtitle_codec_name = ptr::null_mut();
    let mut data_codec_name = ptr::null_mut();

    for codec_name in o.codec_names.iter() {
        let tmp = unsafe { codec_name.u.str } as *mut libc::c_char;
        if codec_name.specifier == "v" {
            video_codec_name = tmp;
        }
        if codec_name.specifier == "a" {
            audio_codec_name = tmp;
        }
        if codec_name.specifier == "s" {
            subtitle_codec_name = tmp;
        }
        if codec_name.specifier == "d" {
            data_codec_name = tmp;
        }
    }

    unsafe {
        if video_codec_name.is_null() {
            ic.video_codec =
                find_codec_or_die(video_codec_name, ffi::AVMediaType_AVMEDIA_TYPE_VIDEO, false);
        }
        if audio_codec_name.is_null() {
            ic.audio_codec =
                find_codec_or_die(audio_codec_name, ffi::AVMediaType_AVMEDIA_TYPE_AUDIO, false);
        }
        if subtitle_codec_name.is_null() {
            ic.subtitle_codec = find_codec_or_die(
                subtitle_codec_name,
                ffi::AVMediaType_AVMEDIA_TYPE_SUBTITLE,
                false,
            );
        }
        if data_codec_name.is_null() {
            ic.data_codec =
                find_codec_or_die(data_codec_name, ffi::AVMediaType_AVMEDIA_TYPE_DATA, false);
        }
    }

    unsafe {
        ic.video_codec_id = if video_codec_name.is_null() {
            ffi::AVCodecID_AV_CODEC_ID_NONE
        } else {
            ic.video_codec.as_ref().unwrap().id
        };

        ic.audio_codec_id = if audio_codec_name.is_null() {
            ffi::AVCodecID_AV_CODEC_ID_NONE
        } else {
            ic.audio_codec.as_ref().unwrap().id
        };

        ic.subtitle_codec_id = if subtitle_codec_name.is_null() {
            ffi::AVCodecID_AV_CODEC_ID_NONE
        } else {
            ic.subtitle_codec.as_ref().unwrap().id
        };

        ic.data_codec_id = if data_codec_name.is_null() {
            ffi::AVCodecID_AV_CODEC_ID_NONE
        } else {
            ic.data_codec.as_ref().unwrap().id
        };
    }
    ic.flags |= ffi::AVFMT_FLAG_NONBLOCK as libc::c_int;
    if o.bitexact != 0 {
        ic.flags |= ffi::AVFMT_FLAG_BITEXACT as libc::c_int;
    }
    ic.interrupt_callback = INT_CB;

    let scan_all_pmts_s = CString::new("scan_all_pmts").unwrap();
    let scan_all_pmts_set = if !unsafe {
        ffi::av_dict_get(
            o.g.format_opts,
            scan_all_pmts_s.as_ptr(),
            ptr::null(),
            ffi::AV_DICT_MATCH_CASE as i32,
        )
    }
    .is_null()
    {
        let one = CString::new("1").unwrap();
        unsafe {
            ffi::av_dict_set(
                &mut o.g.format_opts as *mut _,
                scan_all_pmts_s.as_ptr(),
                one.as_ptr(),
                ffi::AV_DICT_DONT_OVERWRITE as i32,
            );
        };
        true
    } else {
        false
    };
    // open the input file with generic avformat function
    let filename_s = CString::new(filename).unwrap();
    let err = unsafe {
        ffi::avformat_open_input(
            &mut (ic as *mut _) as *mut _,
            filename_s.as_ptr(),
            file_iformat_ptr,
            &mut o.g.format_opts as *mut _,
        )
    };
    if err < 0 {
        print_error(filename, err);
        if err == AVERROR_PROTOCOL_NOT_FOUND {
            error!("Did you mean file:{}?", filename);
        }
        panic!()
    }
    if scan_all_pmts_set {
        unsafe {
            ffi::av_dict_set(
                &mut o.g.format_opts as *mut _,
                scan_all_pmts_s.as_ptr(),
                ptr::null(),
                ffi::AV_DICT_MATCH_CASE as i32,
            );
        }
    }
    unsafe {
        ffmpeg::remove_avoptions(&mut o.g.format_opts, o.g.codec_opts);
        ffmpeg::assert_avoptions(o.g.format_opts);
    }
    let ic_streams = unsafe { slice::from_raw_parts(ic.streams, ic.nb_streams as usize) };
    for stream in ic_streams {
        unsafe {
            choose_decoder(o, ic, *stream);
        }
    }

    if unsafe { find_stream_info } != 0 {
        let opts_ptr = unsafe { cmdutils::setup_find_stream_info_opts(ic, o.g.codec_opts) };
        let orig_nb_streams = ic.nb_streams;
        // If not enough info to get the stream parameters, we decode the
        // first frames to get it. (used in mpeg case for example)
        let ret = unsafe { ffi::avformat_find_stream_info(ic, opts_ptr) };

        let opts = unsafe { slice::from_raw_parts_mut(opts_ptr, orig_nb_streams as _) };
        for opt in opts.iter_mut() {
            unsafe { ffi::av_dict_free(opt) };
        }
        unsafe { ffi::av_freep(opts_ptr as *mut c_void) };

        if ret < 0 {
            error!("{}: could not find codec parameters", filename);
            if ic.nb_streams == 0 {
                unsafe { ffi::avformat_close_input(&mut (ic as *mut _) as *mut _) };
                panic!();
            }
        }
    }

    if o.start_time != AV_NOPTS_VALUE && o.start_time_eof != AV_NOPTS_VALUE {
        error!("Cannot use -ss and -sseof both, using -ss for {}", filename);
        o.start_time_eof = AV_NOPTS_VALUE;
    }

    if o.start_time_eof != AV_NOPTS_VALUE {
        if o.start_time_eof >= 0 {
            error!("-sseof value must be negative; aborting");
            panic!()
        }
        if ic.duration > 0 {
            o.start_time = o.start_time_eof + ic.duration;
            if o.start_time < 0 {
                error!(
                    "-sseof value seeks to before start of file {}; ignored",
                    filename
                );
                o.start_time = AV_NOPTS_VALUE;
            }
        } else {
            error!("Cannot use -sseof, duration of {} not known", filename);
        }
    }
    let mut timestamp = if o.start_time == AV_NOPTS_VALUE {
        0
    } else {
        o.start_time
    };
    // add the stream start time
    if o.seek_timestamp == 0 && ic.start_time != AV_NOPTS_VALUE {
        timestamp += ic.start_time;
    }

    // if seeking requested, we execute it
    if o.start_time != AV_NOPTS_VALUE {
        let mut seek_timestamp = timestamp;

        if unsafe { ic.iformat.as_ref() }.unwrap().flags & ffi::AVFMT_SEEK_TO_PTS as i32 == 0 {
            let ic_streams = unsafe { slice::from_raw_parts(ic.streams, ic.nb_streams as _) };
            if let Some(_) = ic_streams.into_iter().find(|ic_stream|
                unsafe {ic_stream.as_ref().unwrap().codecpar.as_ref().unwrap().video_delay} != 0) {
                seek_timestamp -= (3 * ffi::AV_TIME_BASE / 23) as i64;
            }
        }

        if unsafe { ffi::avformat_seek_file(ic, -1, i64::MIN, seek_timestamp, seek_timestamp, 0) }
            < 0
        {
            error!(
                "{}: could not seek to position {:.3?}",
                filename,
                timestamp as f64 / ffi::AV_TIME_BASE as f64
            );
        }
    }

    // update the current parameters so that they match the one of the input stream
    unimplemented!()
}

unsafe fn choose_decoder(
    o: *mut OptionsContext,
    s: *mut ffi::AVFormatContext,
    st: *mut ffi::AVStream,
) -> *mut ffi::AVCodec {
    unimplemented!()
}

unsafe fn find_codec_or_die(
    name: *const libc::c_char,
    ty: ffi::AVMediaType,
    encoder: bool,
) -> *mut ffi::AVCodec {
    let codec_string = if encoder { "encoder" } else { "decoder" };
    let mut codec = if encoder {
        ffi::avcodec_find_encoder_by_name(name)
    } else {
        ffi::avcodec_find_decoder_by_name(name)
    };
    if codec.is_null() {
        if let Some(desc) = { ffi::avcodec_descriptor_get_by_name(name).as_ref() } {
            codec = if encoder {
                ffi::avcodec_find_encoder(desc.id)
            } else {
                ffi::avcodec_find_decoder(desc.id)
            };
            if let Some(codec) = { codec.as_ref() } {
                let codec_name = { CStr::from_ptr(codec.name) };
                let desc_name = { CStr::from_ptr(codec.name) };
                debug!(
                    "Matched {} '{}' for codec '{}'.",
                    codec_string,
                    codec_name.to_string_lossy(),
                    desc_name.to_string_lossy()
                );
            }
        }
    }
    let name = CStr::from_ptr(name);
    match codec.as_ref() {
        Some(codec) => {
            if codec.type_ != ty {
                error!("Invalid {} type '{}'", codec_string, name.to_string_lossy());
                panic!()
            }
        }
        None => {
            error!("Unknown {} '{}'", codec_string, name.to_string_lossy());
            panic!();
        }
    }
    codec
}

fn open_output_file(o: &mut OptionsContext, filename: &str) -> isize {
    unimplemented!()
}

fn init_complex_filters() {
    unimplemented!()
}

fn check_filter_outputs() {
    unimplemented!()
    /*
        int i;
        for (i = 0; i < nb_filtergraphs; i++) {
            int n;
            for (n = 0; n < filtergraphs[i]->nb_outputs; n++) {
                OutputFilter *output = filtergraphs[i]->outputs[n];
                if (!output->ost) {
                    av_log(NULL, AV_LOG_FATAL, "Filter %s has an unconnected output\n", output->name);
                    exit_program(1);
                }
            }
        }
    */
}

pub fn ffmpeg_parse_options(args: &[String]) {
    // IMPROVEMENT move `init_parse_context(octx, groups)` out of split_commandline() and inline it.
    let mut octx = OptionParseContext {
        groups: (&*GROUPS)
            .iter()
            .map(|group| OptionGroupList {
                group_def: group,
                groups: vec![],
            })
            .collect(),
        global_opts: OptionGroup::new_global(),
        cur_group: OptionGroup::new_anonymous(),
    };

    split_commandline(&mut octx, &args, &*OPTIONS, &*GROUPS).unwrap();
    println!("{:#?}", octx);
    parse_optgroup(None, &octx.global_opts).unwrap();

    open_files(
        &mut octx.groups[OptGroup::GroupInFile as usize],
        "input",
        open_input_file,
    )
    .unwrap();

    init_complex_filters();

    open_files(
        &mut octx.groups[OptGroup::GroupOutFile as usize],
        "output",
        open_output_file,
    )
    .unwrap();

    check_filter_outputs();
}
