//! 简易的命令行程序，主要用来获取epub的元数据，暂不支持修改元数据
//!
//! tool -i file.epub get-cover 1.jpg
//!

use std::{env, fs, io::Write};

use arg::{ArgOption, ArgOptionGroup, OptionDef, OptionType};
use command::{BookInfoGetter, GetChapter, GetCover, GetImage, NavScanner};
use iepub::{reader::read_from_file, EpubBook, EpubError};

mod arg;
mod command;
mod log;

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

macro_rules! register_command {
    ($($cmd_type:ident),+ ) => {
        fn create_command_option_def() -> Vec<arg::CommandOptionDef> {
            vec![
            $(
            $cmd_type::def(),
            )*
            ]

        }

        fn support_command() -> Vec<Box<dyn Command>> {
            vec![
                $(
                    Box::new($cmd_type::default()),
                )*
            ]
        }
    };
}

// 注册子命令
register_command!(GetCover,BookInfoGetter,NavScanner,GetImage,GetChapter);

trait Command {
    ///
    /// 命令
    ///
    fn name(&self) -> String;

    ///
    /// 执行命令
    ///
    fn exec(
        &self,
        book: &mut EpubBook,
        global_opts: &[ArgOption],
        opts: &[ArgOption],
        args: &[String],
    );

    // fn def()->arg::CommandOptionDef;
}

fn main() {
    let mut s: Vec<String> = env::args().collect();
    let exe_file_name = s.remove(0); //把第一个参数去掉
    let mut e = Err(EpubError::Unknown);
    let arg = arg::parse_arg(s, create_option_def(), create_command_option_def()).unwrap();

    if arg.opts.iter().find(|s| s.key == "h").is_some() {
        println!(
            "Usage: {} [options...] [command] [command options...] ",
            exe_file_name
        );
        println!("Example: {} -i input.epub get-cover out.jpg", exe_file_name);
        println!("");
        for ele in create_option_def() {
            println!("-{:10} {}", ele.key, ele.desc);
        }
        println!("");
        println!("supported sub command:");
        println!("");
        for ele in create_command_option_def() {
            println!("{:20} {}", ele.command, ele.desc);
        }
        return;
    }

    let _ = log::is_enable_log(&arg.opts);

    for ele in &arg.opts {
        if ele.key == "i" {
            msg!("reading file ");
            // 读取数据文件
            e = read_from_file(ele.value.as_ref().unwrap().as_str());
            msg!("readed file ");
        }
    }

    let global_opts = arg.opts.as_slice();

    let commands = support_command();
    // 执行 command
    for ele in arg.group {
        let m = commands.iter().find(|s| s.name() == ele.command);
        if let Some(com) = m {
            if ele.opts.iter().find(|s| s.key == "h").is_some() {
                if let Some(def) = create_command_option_def()
                    .iter()
                    .find(|s| s.command == com.name())
                {
                    println!(
                        "Usage: {} {} {}",
                        exe_file_name,
                        com.name(),
                        if def.support_args != 0 {
                            "[file_path]"
                        } else {
                            ""
                        }
                    );
                    for ele in &def.opts {
                        println!("-{:10} {}", ele.key, ele.desc);
                    }
                }

                continue;
            }
            let book = e.as_mut().unwrap();
            com.exec(book, global_opts, &ele.opts, &ele.args);
        }
    }
}
