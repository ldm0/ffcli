mod commands;
mod types;

use env_logger;
use log::{debug, error};
use std::env;

use commands::{GROUPS, OPTIONS};
use libc::c_void;
use types::{
    OptionDef, OptionFlag, OptionGroup, OptionGroupDef, OptionGroupList, OptionKV, OptionOperation,
    OptionParseContext, OptionsContext, SpecifierOpt, SpecifierOptValue,
};

enum OptGroup {
    GroupOutfile = 0,
    GroupInfile = 1,
}

fn main() {
    env_logger::init();
    // Think: May change to use &str later what about none-UTF8 args?
    let args: Vec<String> = env::args().collect();
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
    parse_opt_group(None, &octx.global_opts).unwrap();
}

/// This function accepts moved Option value with the OptionsContext it references to unchanged.
fn parse_opt_group<'ctxt>(
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
                write_option(&mut optctx, o.opt, &o.key, &o.val);
                Ok(())
            }
        })
        .collect::<Result<Vec<_>, ()>>()?;
    debug!("Successfully parsed a group of options.");
    Ok(())
}

/// If failed, panic with some description.
/// TODO: change this function to return  Result later
fn write_option(optctx: &mut Option<&mut OptionsContext>, po: &OptionDef, opt: &str, arg: &str) {
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
        // *dst = parse_number().expect("expect a number");
    } else if po.flags.contains(OptionFlag::OPT_INT64) {
    } else if po.flags.contains(OptionFlag::OPT_TIME) {
    } else if po.flags.contains(OptionFlag::OPT_FLOAT) {
    } else if po.flags.contains(OptionFlag::OPT_DOUBLE) {
    } else if unsafe { po.u.off } != 0 {
        //po.u.func_arg()
    }
    if po.flags.contains(OptionFlag::OPT_EXIT) {
        panic!("exit as required");
    }
}

// TODO the Err in returned Result need to be a ERROR enum
fn split_commandline<'ctxt, 'global>(
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

        print!("Reading option '{}' ...", opt);

        if opt == "--" {
            dashdash = Some(optindex);
            continue;
        }

        // unnamed group separators, e.g. output filename
        if !opt.starts_with('-') || opt.len() <= 1 || dashdash == Some(optindex - 1) {
            // IMPROVEMENT original FFmpeg use 0 rather than enum value here.
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
            // IMPROVEMENT original FFmpeg uses GET_ARG here, but it
            // actually will never throws error
            let arg = if po
                .flags
                .intersects(OptionFlag::OPT_EXIT | OptionFlag::HAS_ARG)
            {
                // Optional argument, e.g. -h
                let arg = &argv[optindex];
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
        /*
        if let Some(opt) = argv.get(optindex) {
            unimplemented!();
        }
        */

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

    if !octx.cur_group.opts.is_empty() {
        // actually or (codec_opts || format_opts || resample_opts) but currently haven't implement this.
        debug!("Trailing option(s) found in the command: may be ignored.");
    }

    debug!("Finished splitting the commandline.");
    Ok(())
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

    // FUTURE FEATURE: initialization for codec_opts

    octx.groups[group_idx].groups.push(new_group);

    // FUTURE FEATURE: call init_opts()

    octx.cur_group = OptionGroup::new_anonymous();
}

fn find_option<'global>(
    options: &'global [OptionDef<'global>],
    name: &str,
) -> Option<&'global OptionDef<'global>> {
    let mut splits = name.split(':');
    let name = match splits.next() {
        Some(x) => x,
        None => return None,
    };
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
