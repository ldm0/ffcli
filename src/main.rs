mod commands;
mod types;

use std::env;

use commands::{GROUPS, OPTIONS};
use types::{
    OptionDef, OptionFlag, OptionGroup, OptionGroupDef, OptionGroupList, OptionKV, OptionOperation,
    OptionParseContext,
};

enum OptGroup {
    GROUP_OUTFILE = 0,
    GROUP_INFILE = 1,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut octx: OptionParseContext = Default::default();
    split_commandline(&mut octx, &args, &OPTIONS, &GROUPS).unwrap();
}

fn split_commandline(
    octx: &mut OptionParseContext,
    args: &[String],
    options: &'static [OptionDef],
    groups: &'static [OptionGroupDef],
) -> Result<(), ()> {
    let argv = args;
    let argc = argv.len();

    init_parse_context(octx, groups);

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

        /* unnamed group separators, e.g. output filename */
        if !opt.starts_with('-') || opt.len() <= 1 || dashdash == Some(optindex - 1) {
            finish_group(octx, OptGroup::GROUP_OUTFILE as usize, opt);
            println!(
                " matched as {}.",
                groups[OptGroup::GROUP_OUTFILE as usize].name
            );
            continue;
        }

        // Jump over prefix `-`
        let opt = opt.get(1..).unwrap();

        /* named group separators, e.g. -i */
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

        /* normal options */
        if let Some(po) = find_option(options, opt) {
            let arg: Option<&str> = if po.flags.contains(OptionFlag::OPT_EXIT) {
                /* optional argument, e.g. -h */
                let arg: Option<&str> = match argv.get(optindex) {
                    // This is pretty strange :-/.
                    // We cannot auto-gen `Option<&str>` from `Option<&String>`.
                    // We need to help the compiler manually.
                    Some(x) => Some(x),
                    None => None,
                };
                optindex += 1;
                arg
            } else if po.flags.contains(OptionFlag::HAS_ARG) {
                let arg = match argv.get(optindex) {
                    Some(arg) => arg,
                    None => return Err(()),
                };
                optindex += 1;
                Some(arg)
            } else {
                Some("1")
            };
            add_opt(octx, po, opt, arg);
            println!(
                " matched as option '{}' ({}) with argument '{:?}'.",
                po.name, po.help, arg
            );
            continue;
        }

        /* AVOptions */
        /*
        if let Some(opt) = argv.get(optindex) {
            unimplemented!();
        }
        */

        /* boolean -nofoo options */
        if opt.starts_with("no") {
            if let Some(opt) = opt.get(2..) {
                if let Some(po) = find_option(options, opt) {
                    if po.flags.contains(OptionFlag::OPT_BOOL) {
                        println!(
                            " matched as option '{}' ({}) with argument 0.",
                            po.name, po.help
                        );
                        continue;
                    }
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

fn init_parse_context(octx: &mut OptionParseContext, groups: &'static [OptionGroupDef]) {
    static GLOBAL_GROUP: OptionGroupDef = OptionGroupDef {
        name: "global",
        sep: None,
        flags: OptionFlag::NONE,
    };
    octx.groups = groups
        .iter()
        .map(|group| OptionGroupList {
            group_def: Some(group),
            ..Default::default()
        })
        .collect();
    octx.global_opts.group_def = Some(&GLOBAL_GROUP);
}

fn match_group_separator(groups: &'static [OptionGroupDef], opt: &str) -> Option<usize> {
    for (i, optdef) in groups.iter().enumerate() {
        if optdef.sep == Some(opt) {
            return Some(i);
        }
    }
    None
}

/// Finish parsing an option group. Move current parsing group into specific group list
/// # Parameters
/// `group_idx`     which group definition should this group belong to
/// `arg`           argument of the group delimiting option
fn finish_group(octx: &mut OptionParseContext, group_idx: usize, arg: &str) {
    octx.groups[group_idx].groups.push(octx.cur_group.clone());
    octx.cur_group = Default::default();
    // TODO: initialization for codec_opts... and call init_opts()
}

fn find_option(options: &'static [OptionDef], name: &str) -> Option<&'static OptionDef> {
    let mut splits = name.split(':');
    let name = match splits.next() {
        Some(x) => x,
        None => return None,
    };
    for option_def in options {
        if option_def.name == name {
            return Some(option_def);
        }
    }
    None
}

/// Add an option instance to currently parsed group.
fn add_opt(octx: &mut OptionParseContext, opt: &'static OptionDef, key: &str, val: Option<&str>) {
    let global = opt.flags
        & (OptionFlag::OPT_PERFILE | OptionFlag::OPT_SPEC | OptionFlag::OPT_OFFSET)
        == Default::default();
    let g = if global {
        &mut octx.global_opts
    } else {
        &mut octx.cur_group
    };
    g.opts.push(OptionKV {
        opt: opt,
        key: key.to_owned(),
        val: val.map(|x| x.to_owned()),
    })
}
