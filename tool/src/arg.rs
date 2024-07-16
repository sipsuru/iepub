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

macro_rules! parse_err {
    ($($arg:tt)*) => {{
        #[cfg(not(test))]
        {
            eprintln!($($arg)*);
            std::process::exit(1);
        }
        #[cfg(test)]
        panic!($($arg)*);
        
    }};

}

#[derive(Debug, Default)]
pub(crate) struct ArgOption {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
    /// array 时填充该值
    pub(crate) values: Option<Vec<String>>,
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
    /// 是否必需
    pub(crate) required: bool,
}
impl OptionDef {
    pub(crate) fn create(key: &str, desc: &str, t: OptionType, required: bool) -> Self {
        OptionDef {
            key: key.to_string(),
            desc: desc.to_string(),
            _type: t,
            required,
        }
    }

    pub(crate) fn over() -> Self {
        OptionDef::create("y", "覆盖已存在文件", OptionType::NoParamter, false)
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

/// 处理字符串，去除可能存在的引号
fn trim_arg(value: String) -> String {
    if value.starts_with("\"") && value.ends_with("\"") {
        return String::from(&value[1..value.len() - 1]);
    }
    if value.starts_with("'") && value.ends_with("'") {
        return String::from(&value[1..value.len() - 1]);
    }
    // 还有转义之类的，这里不考虑了，实在写不完了
    value
}

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
                        ..Default::default()
                    })
                } else {
                    if ele == "-h" {
                        let _ =
                            get_current_opts(&mut arg, current, current_command).is_some_and(|f| {
                                f.push(ArgOption {
                                    key: "h".to_string(),
                                    value: None,
                                    ..Default::default()
                                });
                                true
                            });
                        continue;
                    }
                    parse_err!(
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
                    ..Default::default()
                })
            } else if ele == "-h" {
                if ele == "-h" {
                    let _ = get_current_opts(&mut arg, current, current_command).is_some_and(|f| {
                        f.push(ArgOption {
                            key: "h".to_string(),
                            value: None,
                            ..Default::default()
                        });
                        true
                    });
                    continue;
                }
            } else {
                parse_err!("unsupport args {}", ele);
            }
        } else if current.is_some() {
            // 解析当前的参数
            match current.unwrap()._type {
                OptionType::String => {
                    let v = get_current_opts(&mut arg, current, current_command);
                    if let Some(rv) = v {
                        let index = rv.len() - 1;
                        rv.get_mut(index).unwrap().value = Some(trim_arg(ele));
                    }
                }
                OptionType::Array => {
                    let v = get_current_opts(&mut arg, current, current_command);
                    if let Some(rv) = v {
                        let index = rv.len() - 1;
                        if rv.get_mut(index).unwrap().values.is_none() {
                            rv.get_mut(index).unwrap().values = Some(vec![trim_arg(ele)]);
                        } else {
                            rv.get_mut(index)
                                .unwrap()
                                .values
                                .as_mut()
                                .unwrap()
                                .push(trim_arg(ele));
                        }
                    }
                }
                _ => {}
            }
            current = None;
        } else if current_command.map_or_else(|| false, |com| com.support_args != 0) {
            // 正在解析子命令，且子命令支持arg
            let index = arg.group.len() - 1;
            let group = arg.group.get_mut(index).unwrap();
            if (group.args.len() as i32) == current_command.unwrap().support_args {
                parse_err!(
                    "arg count mush less than {}",
                    current_command.unwrap().support_args
                )
            }

            group.args.push(trim_arg(ele));
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
                parse_err!("unsupported command or arg {}", ele);
            }
        }
    }

    // 校验参数
    if arg.opts.iter().find(|f| f.key == "h").is_none() {
        for ele in &option_def {
            if ele.required {
                let a = arg.opts.iter().find(|s| s.key == ele.key);
                if a.is_none() {
                    parse_err!("grobal arg -{} is required", ele.key);
                } else {
                    match ele._type {
                        OptionType::Array => {
                            if !a
                                .unwrap()
                                .values
                                .as_ref()
                                .map(|f| !f.is_empty())
                                .unwrap_or(false)
                            {
                                parse_err!("grobal arg -{} value is required", ele.key);
                            }
                        }
                        OptionType::String => {
                            if a.unwrap().value.as_ref().is_none() {
                                parse_err!("grobal arg -{} value is required", ele.key);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    for ele in &command_option_def {
        let group = arg.group.iter().find(|f| f.command == ele.command);
        if group.is_none() {
            continue;
        }
        if group.unwrap().opts.iter().find(|f| f.key == "h").is_some() {
            continue;
        }

        for opt in &ele.opts {
            if opt.required {
                let a = group.unwrap().opts.iter().find(|s| s.key == opt.key);
                if a.is_none() {
                    parse_err!("command {} arg -{} is required", ele.command, opt.key);
                } else {
                    match opt._type {
                        OptionType::Array => {
                            if !a
                                .unwrap()
                                .values
                                .as_ref()
                                .map(|f| !f.is_empty())
                                .unwrap_or(false)
                            {
                                parse_err!(
                                    "command {} arg -{} value is required",
                                    ele.command,
                                    opt.key
                                );
                            }
                        }
                        OptionType::String => {
                            if a.unwrap().value.is_none() {
                                parse_err!(
                                    "command {} arg -{} value is required",
                                    ele.command,
                                    opt.key
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(arg)
}

#[cfg(test)]
mod tests {
    use crate::{
        arg::{self, OptionDef, OptionType},
        command::{BookInfoGetter, GetChapter, GetCover, NavScanner},
    };

    use super::parse_arg;

    /// 支持的全局参数
    fn create_option_def() -> Vec<OptionDef> {
        vec![
            OptionDef {
                key: String::from("i"),
                _type: OptionType::String,
                desc: "输入文件，epub".to_string(),
                required: false,
            },
            OptionDef {
                // 覆盖文件
                key: String::from("y"),
                _type: OptionType::NoParamter,
                desc: "全局覆盖输出文件选项".to_string(),
                required: false,
            },
            OptionDef {
                // 日志输出
                key: String::from("l"),
                _type: OptionType::NoParamter,
                desc: "打开终端日志输出".to_string(),
                required: false,
            },
        ]
    }
    /// 支持的子命令
    fn create_command_option_def() -> Vec<arg::CommandOptionDef> {
        vec![
            GetCover::def(),
            BookInfoGetter::def(),
            NavScanner::def(),
            GetChapter::def(),
        ]
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
                    "cover.jpg".to_string()
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );
        assert_eq!(
            r#"Arg { opts: [ArgOption { key: "i", value: Some("fis"), values: None }], group: [ArgOptionGroup { command: "get-cover", opts: [], args: ["cover.jpg"] }] }"#,
            m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "-i".to_string(),
                    "fis".to_string(),
                    "get-cover".to_string(),
                    "cover.jpg".to_string(),
                    "get-cover".to_string(),
                    "cover.jpg".to_string(),
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        assert_eq!(
            "Arg { opts: [ArgOption { key: \"i\", value: Some(\"fis\"), values: None }], group: [ArgOptionGroup { command: \"get-cover\", opts: [], args: [\"cover.jpg\", \"get-cover\", \"cover.jpg\"] }] }"
            ,m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "get-cover".to_string(),
                    "cover.jpg".to_string(),
                    "cover.jpg".to_string(),
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        assert_eq!(
            "Arg { opts: [], group: [ArgOptionGroup { command: \"get-cover\", opts: [], args: [\"cover.jpg\", \"cover.jpg\"] }] }"
            ,m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "get-cover".to_string(),
                    "-y".to_string(),
                    "cover.jpg".to_string(),
                    
                ],
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );
        assert_eq!(
            "Arg { opts: [], group: [ArgOptionGroup { command: \"get-cover\", opts: [ArgOption { key: \"y\", value: None, values: None }], args: [\"cover.jpg\"] }] }"
            ,m
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
            "Arg { opts: [ArgOption { key: \"h\", value: None, values: None }], group: [] }"
            ,m
        );

        m = format!(
            "{:?}",
            parse_arg(
                vec![
                    "-l",
                    "-i",
                    "/app/魔女之旅.epub",
                    "get-info",
                    "-title",
                    "-author",
                    "-title",
                    "-isbn",
                    "-publisher",
                    "nav",
                    "-s",
                    "get-chapter",
                    "-c",
                    "022.第二十一卷/0290.xhtml",
                    "-b"
                ]
                .iter()
                .map(|f| f.to_string())
                .collect(),
                create_option_def(),
                create_command_option_def()
            )
            .unwrap()
        );

        println!("m={:?}",m);
    }
}
