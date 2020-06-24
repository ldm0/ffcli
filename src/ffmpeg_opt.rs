use libc::c_void;
use log::{debug, error};

use crate::{
    cmdutils::{
        parse_optgroup, split_commandline, OptionDef, OptionFlag, OptionGroup, OptionGroupDef,
        OptionGroupList, OptionKV, OptionOperation, OptionParseContext, SpecifierOpt,
        SpecifierOptValue,
    },
    options::{GROUPS, OPTIONS},
    ffmpeg::OptionsContext,
};

enum OptGroup {
    GroupOutFile = 0,
    GroupInFile = 1,
}

fn open_files(l: &mut OptionGroupList, inout: &str, open_file: fn(&mut OptionsContext, &str) -> isize) -> Result<(), ()>{
    for g in l.groups.iter() {
        let mut o = OptionsContext::new(Some(g));
        if let Err(ret) = parse_optgroup(Some(&mut o), g) {
            error!("Error parsing options for {} file {}.", inout, g.arg);
            return Err(ret);
        }
        debug!("Opening an {} file: {}.", inout, g.arg);
        let ret = open_file(&mut o, &g.arg);
        if ret < 0 {
            error!("Error opening {} file {}.\n",
                   inout, g.arg);
            return Err(());
        }
        debug!("Successfully opened the file.");
    }
    Ok(())
}

fn open_input_file(o: &mut OptionsContext, filename: &str) -> isize {
    unimplemented!()
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
    open_files(&mut octx.groups[OptGroup::GroupInFile as usize], "input", open_input_file).unwrap();

    init_complex_filters();

    open_files(&mut octx.groups[OptGroup::GroupOutFile as usize], "output", open_output_file).unwrap();

    check_filter_outputs();
}
