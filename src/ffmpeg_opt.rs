use libc::c_void;
use log::{debug, error};

use crate::{
    cmdutils::{
        parse_opt_group, split_commandline, OptionDef, OptionFlag, OptionGroup, OptionGroupDef,
        OptionGroupList, OptionKV, OptionOperation, OptionParseContext, SpecifierOpt,
        SpecifierOptValue,
    },
    options::{GROUPS, OPTIONS},
};

enum OptGroup {
    GroupOutfile = 0,
    GroupInfile = 1,
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
    parse_opt_group(None, &octx.global_opts).unwrap();
}
