# FFCLI

This project is meant to build FFmpeg command line arguments parser. This is roughly a ported ffmpeg.

`AVOptions` have not been supported.

You can test by using `cargo test`.

To build it, you should compile the FFmpeg then use the `PKG_CONFIG_PATH="$HOME/ffmpeg_build/lib/pkgconfig" cargo build` to build it(where `PKG_CONFIG_PATH` points to `*.pc` files in the FFmpeg build result).

And you can use `RUST_LOG=debug cargo run -- -i input -c:v libx264 -profile:v main -preset:v fast -level 3.1 -x264opts crf=18 ` to check program outputs (I changed the `av_log`s in FFmpeg's source code to `log::{debug, error}`, so you will see roughly the same debug message when specific parameters are given.)

Current output:
```rust
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Splitting the commandline.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-i' ...
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as input url with argument 'input'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-c:v' ...
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as option 'c' (codec name) with argument '"libx264"'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-profile:v' ...
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as option 'profile' (set profile) with argument '"main"'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-preset:v' ...
[2020-06-04T18:15:49Z ERROR ffcli::cmdutils] opt_default() heavily uses functions in the libavutil, currently assume preset:v: fast is a valid AVOption pair.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as AVOption 'preset:v' with argument 'fast'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-level' ...
[2020-06-04T18:15:49Z ERROR ffcli::cmdutils] opt_default() heavily uses functions in the libavutil, currently assume level: 3.1 is a valid AVOption pair.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as AVOption 'level' with argument '3.1'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Reading option '-x264opts' ...
[2020-06-04T18:15:49Z ERROR ffcli::cmdutils] opt_default() heavily uses functions in the libavutil, currently assume x264opts: crf=18 is a valid AVOption pair.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils]  matched as AVOption 'x264opts' with argument 'crf=18'.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Trailing option(s) found in the command: may be ignored.
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Finished splitting the commandline.
OptionParseContext {
    global_opts: OptionGroup {
        group_def: OptionGroupDef {
            name: "global",
            sep: None,
            flags: NONE,
        },
        arg: "",
        opts: [],
    },
    groups: [
        OptionGroupList {
            group_def: OptionGroupDef {
                name: "output url",
                sep: None,
                flags: OPT_OUTPUT,
            },
            groups: [],
        },
        OptionGroupList {
            group_def: OptionGroupDef {
                name: "input url",
                sep: Some(
                    "i",
                ),
                flags: OPT_INPUT,
            },
            groups: [
                OptionGroup {
                    group_def: OptionGroupDef {
                        name: "input url",
                        sep: Some(
                            "i",
                        ),
                        flags: OPT_INPUT,
                    },
                    arg: "input",
                    opts: [],
                },
            ],
        },
    ],
    cur_group: OptionGroup {
        group_def: OptionGroupDef {
            name: "never_used",
            sep: None,
            flags: NONE,
        },
        arg: "",
        opts: [
            OptionKV {
                opt: OptionDef {
                    name: "c",
                    help: "codec name",
                    argname: Some(
                        "codec",
                    ),
                    flags: HAS_ARG | OPT_STRING | OPT_SPEC | OPT_INPUT | OPT_OUTPUT,
                    u: (Union)OptionOperation {
                        val: 96,
                    },
                },
                key: "c:v",
                val: "libx264",
            },
            OptionKV {
                opt: OptionDef {
                    name: "profile",
                    help: "set profile",
                    argname: Some(
                        "profile",
                    ),
                    flags: HAS_ARG | OPT_EXPERT | OPT_PERFILE | OPT_OUTPUT,
                    u: (Union)OptionOperation {
                        val: 140697315024544,
                    },
                },
                key: "profile:v",
                val: "main",
            },
        ],
    },
}
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Parsing a group of options: global .
[2020-06-04T18:15:49Z DEBUG ffcli::cmdutils] Successfully parsed a group of options.
```

This program is under the original license of FFmpeg.
