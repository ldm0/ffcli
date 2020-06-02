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

/// This default implementation is specially used for cur_group before it's
/// refactored into tuple.
impl<'global> default::Default for OptionGroup<'global> {
    fn default() -> Self {
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
