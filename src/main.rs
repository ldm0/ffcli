mod commands;
mod types;

use std::env;

use commands::{GROUPS, OPTIONS};
use types::{
    OptionDef, OptionFlag, OptionGroup, OptionGroupDef, OptionGroupList, OptionKV, OptionOperation,
    OptionParseContext,
};

enum OptGroup {
    GroupOutfile = 0,
    GroupInfile = 1,
}

fn main() {
    // Think: May change to use &str later what about none-UTF8 args?
    let args: Vec<String> = env::args().collect();
    // IMPROVEMENT move `init_parse_context(octx, groups)` out of split_commandline() and inline it.

    static GLOBAL_GROUP: OptionGroupDef = OptionGroupDef {
        name: "global",
        sep: None,
        flags: OptionFlag::NONE,
    };

    let mut octx = OptionParseContext {
        groups: (&*GROUPS)
            .iter()
            .map(|group| OptionGroupList {
                group_def: group,
                groups: vec![],
            })
            .collect(),
        global_opts: OptionGroup {
            group_def: &GLOBAL_GROUP,
            arg: String::new(),
            opts: vec![],
        },
        cur_group: Default::default(),
    };

    split_commandline(&mut octx, &args, &*OPTIONS, &*GROUPS).unwrap();
    parse_opt_group_global(&octx.global_opts);
}

/// Treat original FFmpeg's `parse_opt_group(NULL, _)` as `parse_opt_group_global(_)`
fn parse_opt_group_global<'ctxt>(g: &OptionGroup) -> Result<(), ()> {
    println!(
        "Parsing a group of options: {} {}.",
        g.group_def.name, g.arg,
    );
    for o in g.opts.iter() {
        if !g.group_def.flags.is_empty() && !g.group_def.flags.intersects(o.opt.flags) {
            println!(
                "Option {} ({}) cannot be applied to \
                   {} {} -- you are trying to apply an input option to an \
                   output file or vice versa. Move this option before the \
                   file it belongs to.",
                o.key, o.opt.help, g.group_def.name, g.arg
            );
            return Err(());
        }
        println!(
            "Applying option {} ({}) with argument {}.",
            o.key, o.opt.help, o.val
        );
        write_option_global(o.opt, &o.key, &o.val);
    }

    println!("Successfully parsed a group of options.");
    Ok(())
}

/// If failed, panic with some description.
/// TODO: change this to Result later
fn write_option_global(po: &OptionDef, opt: &str, arg: &str) {
    if po.flags.contains(OptionFlag::OPT_SPEC) {
        unimplemented!();
    }

    if po.flags.contains(OptionFlag::OPT_STRING) {
        unimplemented!()
    } else if po
        .flags
        .contains(OptionFlag::OPT_STRING | OptionFlag::OPT_INT)
    {
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

    println!("Splitting the commandline.");

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
            println!(
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
            println!(
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
                .contains(OptionFlag::OPT_EXIT | OptionFlag::HAS_ARG)
            {
                // Optional argument, e.g. -h
                let arg = &argv[optindex];
                optindex += 1;
                arg
            } else {
                "1"
            };
            add_opt(octx, po, opt, arg);
            println!(
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
                    println!(
                        " matched as option '{}' ({}) with argument 0.",
                        po.name, po.help
                    );
                    continue;
                }
            }
        }

        println!("Unrecognized option '{}'.", opt);
        return Err(());
    }

    if !octx.cur_group.opts.is_empty() {
        // actually or (codec_opts || format_opts || resample_opts) but currently haven't implement this.
        println!("Trailing option(s) found in the command: may be ignored.");
    }

    println!("Finished splitting the commandline.");
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

    octx.cur_group = Default::default();
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
        .contains(OptionFlag::OPT_PERFILE | OptionFlag::OPT_SPEC | OptionFlag::OPT_OFFSET);
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
