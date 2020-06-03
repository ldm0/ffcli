// This is need for those global values in FFmpeg
#![allow(non_upper_case_globals)]
// This will be finally removed, but in development stage it's useful
#![allow(unused_variables)]
use libc::c_void;
use once_cell::sync::Lazy;

use crate::cmdutils::{
    OptionDef, OptionFlag, OptionGroup, OptionGroupDef, OptionGroupList, OptionKV, OptionOperation,
    OptionParseContext,
};

macro_rules! void {
    ($x: expr) => {
        unsafe { &mut $x as *mut _ as *mut c_void }
    };
}


macro_rules! option_operation {
    (dst_ptr => $operation: expr) => {
        OptionOperation {
            dst_ptr: void!($operation),
        }
    };
    (func_arg => $operation: expr) => {
        OptionOperation {
            func_arg: $operation,
        }
    };
    (off => $operation: expr) => {
        OptionOperation {
            off: $operation,
        }
    };
}

macro_rules! option_def {
    ($name: literal, $flags: expr, dst_ptr => $operation: expr, $help: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(dst_ptr => $operation),
            $help, None
        )
    };
    ($name: literal, $flags: expr, func_arg => $operation: expr, $help: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(func_arg => $operation),
            $help, None
        )
    };
    ($name: literal, $flags: expr, off => $operation: expr, $help: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(off => $operation),
            $help, None
        )
    };
    ($name: literal, $flags: expr, dst_ptr => $operation: expr, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(dst_ptr => $operation),
            $help, Some($argname)
        )
    };
    ($name: literal, $flags: expr, func_arg => $operation: expr, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(func_arg => $operation),
            $help, Some($argname)
        )
    };
    ($name: literal, $flags: expr, off => $operation: expr, $help: literal, $argname: literal) => {
        option_def! (
            @inner $name, $flags,
            option_operation!(off => $operation),
            $help, Some($argname)
        )
    };
    (@inner $name: literal, $flags: expr, $u: expr, $help: literal, $argname: expr) => {
        OptionDef {
            name: $name,
            help: $help,
            argname: $argname,
            flags: $flags,
            u: $u,
        }
    };
}

macro_rules! option_group_def {
    ($name: literal) => {
        option_group_def!(@inner $name, None, OptionFlag::NONE)
    };
    ($name: literal, $flags: expr) => {
        option_group_def!(@inner $name, None, $flags)
    };
    ($name: literal, $separator: literal, $flags: expr) => {
        option_group_def!(@inner $name, Some($separator), $flags)
    };
    (@inner $name: literal, $separator: expr, $flags: expr) => {
        OptionGroupDef {
            name: $name,
            sep: $separator,
            flags: $flags,
        }
    }
}

pub static GROUPS: Lazy<[OptionGroupDef; 2]> = Lazy::new(|| {
    [
        option_group_def!("output url", OptionFlag::OPT_OUTPUT),
        option_group_def!("input url", "i", OptionFlag::OPT_INPUT),
    ]
});

/// The options list is in ffmpeg_opt.c originally, but we move it here for cleanness.
pub static OPTIONS: Lazy<[OptionDef; 29]> = Lazy::new(|| {
    [
        // Common options
        option_def!("L",            OptionFlag::OPT_EXIT,               func_arg => show_license,     "show license"),
        option_def!("h",            OptionFlag::OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("?",            OptionFlag::OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("help",         OptionFlag::OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("-help",        OptionFlag::OPT_EXIT,               func_arg => show_help,        "show help", "topic"),
        option_def!("version",      OptionFlag::OPT_EXIT,               func_arg => show_version,     "show version"),
        option_def!("buildconf",    OptionFlag::OPT_EXIT,               func_arg => show_buildconf,   "show build configuration"),
        option_def!("formats",      OptionFlag::OPT_EXIT,               func_arg => show_formats,     "show available formats"),
        option_def!("muxers",       OptionFlag::OPT_EXIT,               func_arg => show_muxers,      "show available muxers"),
        option_def!("demuxers",     OptionFlag::OPT_EXIT,               func_arg => show_demuxers,    "show available demuxers"),
        option_def!("devices",      OptionFlag::OPT_EXIT,               func_arg => show_devices,     "show available devices"),
        option_def!("codecs",       OptionFlag::OPT_EXIT,               func_arg => show_codecs,      "show available codecs"),
        option_def!("decoders",     OptionFlag::OPT_EXIT,               func_arg => show_decoders,    "show available decoders"),
        option_def!("encoders",     OptionFlag::OPT_EXIT,               func_arg => show_encoders,    "show available encoders"),
        option_def!("bsfs",         OptionFlag::OPT_EXIT,               func_arg => show_bsfs,        "show available bit stream filters"),
        option_def!("protocols",    OptionFlag::OPT_EXIT,               func_arg => show_protocols,   "show available protocols"),
        option_def!("filters",      OptionFlag::OPT_EXIT,               func_arg => show_filters,     "show available filters"),
        option_def!("pix_fmts",     OptionFlag::OPT_EXIT,               func_arg => show_pix_fmts,    "show available pixel formats"),
        option_def!("layouts",      OptionFlag::OPT_EXIT,               func_arg => show_layouts,     "show standard channel layouts"),
        option_def!("sample_fmts",  OptionFlag::OPT_EXIT,               func_arg => show_sample_fmts, "show available audio sample formats"),
        option_def!("colors",       OptionFlag::OPT_EXIT,               func_arg => show_colors,      "show available color names"),
        option_def!("loglevel",     OptionFlag::HAS_ARG,                func_arg => opt_loglevel,     "set logging level", "loglevel"),
        option_def!("v",            OptionFlag::HAS_ARG,                func_arg => opt_loglevel,     "set logging level", "loglevel"),
        option_def!("report",       OptionFlag::NONE,                   func_arg => opt_report,       "generate a report"),
        option_def!("max_alloc",    OptionFlag::HAS_ARG,                func_arg => opt_max_alloc,    "set maximum size of a single allocated block", "bytes"),
        option_def!("cpuflags",     OptionFlag::HAS_ARG | OptionFlag::OPT_EXPERT,   func_arg => opt_cpuflags,       "force specific cpu flags", "flags"),
        option_def!("hide_banner",  OptionFlag::OPT_BOOL | OptionFlag::OPT_EXPERT,  dst_ptr => hide_banner,  "do not show program banner", "hide_banner"),
        option_def!("sources",      OptionFlag::OPT_EXIT | OptionFlag::HAS_ARG,     func_arg => show_sources,       "list sources of the input device", "device"),
        option_def!("sinks",        OptionFlag::OPT_EXIT | OptionFlag::HAS_ARG,     func_arg => show_sinks,         "list sinks of the output device", "device"),
    ]
});

static mut hide_banner: bool = false;

fn show_license(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    print!(
        "This is free software; you can redistribute it and/or\n
    modify it under the terms of the GNU Lesser General Public\n
    License as published by the Free Software Foundation; either\n
    version 2.1 of the License, or (at your option) any later version.\n
    \n
    This is distributed in the hope that it will be useful,\n
    but WITHOUT ANY WARRANTY; without even the implied warranty of\n
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU\n
    Lesser General Public License for more details.\n
    \n
    You should have received a copy of the GNU Lesser General Public\n
    License along with this program; if not, write to the Free Software\n
    Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA\n"
    );
    0
}

fn show_help(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<help message>");
    0
}

fn show_version(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<version message>");
    0
}

fn show_buildconf(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<buildconf message>");
    0
}

fn show_formats(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<formats message>");
    0
}

fn show_muxers(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<muxers message>");
    0
}

fn show_demuxers(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<demuxers message>");
    0
}

fn show_devices(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<devices message>");
    0
}

fn show_codecs(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<codecs message>");
    0
}

fn show_decoders(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<decoders message>");
    0
}

fn show_encoders(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<encoders message>");
    0
}

fn show_bsfs(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<bsfs message>");
    0
}

fn show_protocols(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<protocols message>");
    0
}

fn show_filters(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<filers message>");
    0
}

fn show_pix_fmts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<pix_fmts message>");
    0
}

fn show_layouts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<layouts message>");
    0
}

fn show_sample_fmts(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sample_fmts message>");
    0
}

fn show_colors(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<colors message>");
    0
}

fn opt_loglevel(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<loglevel message>");
    0
}

fn opt_report(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<report message>");
    0
}

fn opt_max_alloc(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<max_alloc message>");
    0
}

fn opt_cpuflags(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<cpuflags message>");
    0
}

fn show_sources(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sources message>");
    0
}

fn show_sinks(optctx: *mut c_void, opt: &str, arg: &str) -> i64 {
    println!("<sinks message>");
    0
}

#[cfg(test)]
mod command_tests {
    use super::*;

    fn opt_cpuflags(_: *mut c_void, _: &str, _: &str) -> i64 {
        0
    }

    #[test]
    fn option_def_macro() {
        let opt = option_def!(
            "cpuflags",
            OptionFlag::HAS_ARG | OptionFlag::OPT_EXPERT,
            func_arg => opt_cpuflags,
            "force specific cpu flags",
            "flags"
        );
        // We cannot confirm the address of function pointer though.
        assert_eq!(opt.name, "cpuflags");
        assert_eq!(opt.flags, OptionFlag::HAS_ARG | OptionFlag::OPT_EXPERT);
        assert_eq!(opt.help, "force specific cpu flags");
        assert_eq!(opt.argname, Some("flags"));
    }

    #[test]
    fn option_operation_macro() {
        // Test whether it compiles.
        let _ = option_operation!(func_arg => show_help);
    }
}
