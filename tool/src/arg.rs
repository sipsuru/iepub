//! 命令行参数解析
//!
//! 基本格式如下:
//! exec -C arg1 get-cover -o out.jpg
//!
//! exec代表可执行命令，也就是当前程序
//!
//! -C arg1 全局参数
//!
//! get-cover 子命令
//!
//! -o out.jpg 子命令参数
//!
//! 子命令及其参数可以有多个，将会同时执行
//!

#[derive(Debug, Default)]
pub(crate) struct ArgOption {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
}
#[derive(Debug, Default)]
pub(crate) struct ArgOptionGroup {
    /// 子命令
    pub(crate) command: String,
    /// 子命令参数，指定以 - 开头的参数
    pub(crate) opts: Vec<ArgOption>,
    /// 直接接在子命令之后的
    ///
    /// 例如 get-cover 1.jpg
    ///
    /// 这个1.jpg 就属于 args
    pub(crate) args: Vec<String>,
}
#[derive(Debug, Default)]
pub(crate) struct Arg {
    /// 全局参数
    pub(crate) opts: Vec<ArgOption>,
    /// 子命令
    pub(crate) group: Vec<ArgOptionGroup>,
}
#[derive(Debug)]
pub(crate) enum OptionType {
    /// 数组
    Array,
    /// 没有参数
    NoParamter,
    /// 文件路径
    String,
}

/// 支持的参数
#[derive(Debug)]
pub(crate) struct OptionDef {
    /// 参数名，例如 f 使用时 -f
    pub(crate) key: String,
    /// 参数类型，
    pub(crate) _type: OptionType,
    pub(crate) desc: String,
}
impl OptionDef {
    pub(crate) fn create(key:&str,desc:&str,t:OptionType)->Self{
        OptionDef{
            key:key.to_string(),
            desc:desc.to_string(),
            _type:t
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CommandOptionDef {
    pub(crate) command: String,
    pub(crate) opts: Vec<OptionDef>,

    /// 是否支持args
    ///
    /// 0代表不支持
    /// -1代表支持无限个，此时这种子命令应该放到最后一个，否则可能导致后面的子命令被识别成该命令的args
    /// 其他数字代表支持的个数
    pub(crate) support_args: i32,

    pub(crate) desc: String,
}

// fn get_option_def(key: &str, def: &[OptionDef]) -> Option<OptionDef> {}

pub(crate) fn parse_arg(
    args: Vec<String>,
    option_def: Vec<OptionDef>,
    command_option_def: Vec<CommandOptionDef>,
) -> Result<Arg, String> {
    #[inline]
    fn get_current_opts<'a>(
        arg: &'a mut Arg,
        _current: Option<&'a OptionDef>,
        current_command: Option<&'a CommandOptionDef>,
    ) -> Option<&'a mut Vec<ArgOption>> {
        if current_command.is_some() {
            let index = arg.group.len() - 1;
            let g = &mut arg.group;
            let group = g.get_mut(index).unwrap();
            return Some(&mut group.opts);
        }
        Some(&mut arg.opts)
    }

    let mut arg = Arg::default();

    let mut current: Option<&OptionDef> = None;
    let mut current_command: Option<&CommandOptionDef> = None;
    for ele in args {
        if ele.starts_with("-") {
            let key = &ele[1..];
            // 子命令参数
            if let Some(cc) = current_command {
                // 解析子命令的参数
                let key = &ele[1..];
                // 参数，需要判断
                current = cc.opts.iter().find(|s| s.key == key);

                if current.is_some() {
                    let index = arg.group.len() - 1;
                    let mut group = arg.group.get_mut(index).unwrap();
                    match current.unwrap()._type {
                        OptionType::NoParamter => {
                            // 没有参数，到此为止
                            current = None;
                        }
                        _ => {}
                    }

                    group.opts.push(ArgOption {
                        key: key.to_string(),
                        value: None,
                    })
                } else {
                    if ele == "-h" {
                        let _ =
                            get_current_opts(&mut arg, current, current_command).is_some_and(|f| {
                                f.push(ArgOption {
                                    key: "h".to_string(),
                                    value: None,
                                });
                                true
                            });
                        continue;
                    }
                    panic!(
                        "unsupport args {} for command {}",
                        ele,
                        current_command.unwrap().command
                    );
                }
                continue;
            }

            // 全局参数，需要判断
            current = option_def.iter().find(|s| s.key == key);
            if current.is_some() {
                match current.unwrap()._type {
                    OptionType::NoParamter => {
                        // 没有参数，到此为止
                        current = None;
                    }
                    _ => {}
                }
                arg.opts.push(ArgOption {
                    key: key.to_string(),
                    value: None,
                })
            } else if ele == "-h" {
                if ele == "-h" {
                    let _ = get_current_opts(&mut arg, current, current_command).is_some_and(|f| {
                        f.push(ArgOption {
                            key: "h".to_string(),
                            value: None,
                        });
                        true
                    });
                    continue;
                }
            } else {
                panic!("unsupport args {}", ele);
            }
        } else if current.is_some() {
            // 解析当前的参数
            match current.unwrap()._type {
                OptionType::String => {
                    let v = get_current_opts(&mut arg, current, current_command);
                    if let Some(rv) = v {
                        let index = rv.len() - 1;
                        rv.get_mut(index).unwrap().value = Some(ele);
                    }
                }
                _ => {}
            }
            current = None;
        } else if current_command.map_or_else(|| false, |com| com.support_args != 0) {
            // 正在解析子命令，且子命令支持arg
            let index = arg.group.len() - 1;
            let mut group = arg.group.get_mut(index).unwrap();
            if (group.args.len() as i32) == current_command.unwrap().support_args {
                panic!(
                    "arg count mush less than {}",
                    current_command.unwrap().support_args
                )
            }

            group.args.push(ele);
        } else {
            // 可能是子命令
            current_command = command_option_def.iter().find(|f| f.command == ele);
            if current_command.is_some() {
                arg.group.push(ArgOptionGroup {
                    command: ele,
                    opts: Vec::new(),
                    args: Vec::new(),
                })
            } else {
                panic!("unsupported command or arg {}", ele);
            }
        }
    }

    Ok(arg)
}

#[cfg(test)]
mod tests {
    use crate::arg::{self, OptionDef, OptionType};

    use super::parse_arg;

    /// 支持的全局参数
    fn create_option_def() -> Vec<OptionDef> {
        vec![
            OptionDef {
                key: String::from("i"),
                _type: OptionType::String,
                desc: "输入文件，epub".to_string(),
            },
            OptionDef {
                // 覆盖文件
                key: String::from("y"),
                _type: OptionType::NoParamter,
                desc: "全局覆盖输出文件选项".to_string(),
            },
            OptionDef {
                // 日志输出
                key: String::from("l"),
                _type: OptionType::NoParamter,
                desc: "打开终端日志输出".to_string(),
            },
        ]
    }
    /// 支持的子命令
    fn create_command_option_def() -> Vec<arg::CommandOptionDef> {
        vec![arg::CommandOptionDef {
            command: String::from("get-cover"),
            desc: "提取电子书封面, 例如get-cover 1.jpg，输出到1.jpg".to_string(),
            support_args: -1,
            opts: vec![
                OptionDef {
                    key: String::from("o"),
                    _type: OptionType::String,
                    desc: "输出文件名".to_string(),
                },
                OptionDef {
                    // 覆盖文件
                    key: String::from("y"),
                    _type: OptionType::NoParamter,
                    desc: "是否覆盖输出文件".to_string(),
                },
            ],
        }]
    }
    #[test]
    fn test_parse_args() {
        let mut m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "-i".to_string(),
                    "fis".to_string(),
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string()
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );
        assert_eq!(
            r#"Arg { opts: [ArgOption { key: "i", value: Some("fis") }], group: [ArgOptionGroup { command: "get-cover", opts: [ArgOption { key: "o", value: Some("cover.jpg") }], args: [] }] }"#,
            m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "-i".to_string(),
                    "fis".to_string(),
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        assert_eq!(
            r#"Arg { opts: [ArgOption { key: "i", value: Some("fis") }], group: [ArgOptionGroup { command: "get-cover", opts: [ArgOption { key: "o", value: Some("cover.jpg") }, ArgOption { key: "o", value: Some("cover.jpg") }], args: ["get-cover"] }] }"#,
            m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        assert_eq!(
            r#"Arg { opts: [], group: [ArgOptionGroup { command: "get-cover", opts: [ArgOption { key: "o", value: Some("cover.jpg") }, ArgOption { key: "o", value: Some("cover.jpg") }], args: ["get-cover"] }] }"#,
            m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                    "get-cover".to_string(),
                    "-o".to_string(),
                    "cover.jpg".to_string(),
                    "-y".to_string(),
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );
        assert_eq!(
            r#"Arg { opts: [], group: [ArgOptionGroup { command: "get-cover", opts: [ArgOption { key: "o", value: Some("cover.jpg") }, ArgOption { key: "o", value: Some("cover.jpg") }, ArgOption { key: "y", value: None }], args: ["get-cover"] }] }"#,
            m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec!["-h".to_string(),],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        assert_eq!(
            r#"Arg { opts: [ArgOption { key: "h", value: None }], group: [] }"#,
            m
        );
    }
}
