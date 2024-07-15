use std::io::Write;

use crate::{
    arg::{self, ArgOption, CommandOptionDef, OptionDef, OptionType},
    msg, Command,
};
use iepub::{EpubBook, EpubError, EpubNav};
// 是否覆盖文件
fn is_overiade(arg: &[arg::ArgOption]) -> bool {
    for ele in arg {
        if ele.key == "y" {
            return true;
        }
    }

    false
}

///
/// 获取输入
///
fn get_single_input(message: &str) -> Result<String, EpubError> {
    println!("{}", message);
    use std::io::BufRead;
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    handle.read_line(&mut buffer)?;
    Ok(buffer)
}

#[derive(Default)]
pub(crate) struct BookInfoGetter;
impl BookInfoGetter {
    pub(crate) fn def() -> arg::CommandOptionDef {
        arg::CommandOptionDef {
            command: "get-info".to_string(),
            support_args: 0,
            desc: "提取数据元数据".to_string(),
            opts: vec![
                OptionDef::create("title", "标题", OptionType::NoParamter),
                OptionDef::create("author", "作者", OptionType::NoParamter),
                OptionDef::create("isbn", "isbn", OptionType::NoParamter),
                OptionDef::create("publisher", "出版社", OptionType::NoParamter),
            ],
        }
    }
}
impl Command for BookInfoGetter {
    fn name(&self) -> String {
        "get-info".to_string()
    }

    fn exec(
        &self,
        book: &mut EpubBook,
        _global_opts: &[ArgOption],
        opts: &[ArgOption],
        _args: &[String],
    ) {
        for ele in opts {
            match ele.key.as_str() {
                "title" => println!("{}", book.title()),
                "author" => println!("{}", book.creator().unwrap_or("")),
                "isbn" => println!("{}", book.identifier()),
                "publisher" => println!("{}", book.publisher().unwrap_or("")),
                _ => {}
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct GetCover;
impl GetCover {
    pub(crate) fn def() -> arg::CommandOptionDef {
        arg::CommandOptionDef {
            command: String::from("get-cover"),
            desc: "提取电子书封面, 例如get-cover 1.jpg，输出到1.jpg".to_string(),
            support_args: -1,
            opts: vec![
                // OptionDef {
                //     key: String::from("o"),
                //     _type: OptionType::String,
                // },
                arg::OptionDef {
                    // 覆盖文件
                    key: String::from("y"),
                    _type: arg::OptionType::NoParamter,
                    desc: "是否覆盖输出文件".to_string(),
                },
            ],
        }
    }
}
impl Command for GetCover {
    fn name(&self) -> String {
        "get-cover".to_string()
    }

    fn exec(
        &self,
        book: &mut EpubBook,
        global_opts: &[ArgOption],
        opts: &[ArgOption],
        args: &[String],
    ) {
        let cover = book.cover().unwrap();

        #[inline]
        fn write_file(path: &str, data: &[u8]) {
            let mut fs = std::fs::File::create(path).unwrap();
            fs.write_all(data).unwrap();
        }

        for path in args {
            if std::fs::File::open(path).is_ok() {
                msg!("file {} has exist", path.as_str());
                // 文件已存在，判断有没有覆盖参数，
                if is_overiade(global_opts) || is_overiade(&opts) {
                    write_file(path, cover.data().unwrap());
                } else if get_single_input("Override file？(y/n)")
                    .unwrap()
                    .to_lowercase()
                    == "y"
                {
                    // 询问
                    write_file(path, cover.data().unwrap());
                }
            } else {
                write_file(path, cover.data().unwrap());
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct NavScanner;
impl NavScanner {
    pub(crate) fn def()->CommandOptionDef {
        CommandOptionDef{
            command:"nav".to_string(),
            desc:"导航".to_string(),
            support_args:0,
            opts:Vec::new()
        }
    }
    fn print_nav(&self,dec:i32,nav:&EpubNav){
        self.print_dec(dec);
        println!("{}",nav.title());
        for ele in nav.child() {
            self.print_nav(dec+1, ele);
        }
    }
    fn print_dec(&self,dec:i32){
        for _ in 0..dec {
            print!(" ");
        }
    }
}
impl Command for NavScanner {
    fn name(&self) -> String {
        "nav".to_string()
    }


    fn exec(
        &self,
        book: &mut EpubBook,
        _global_opts: &[ArgOption],
        _opts: &[ArgOption],
        _args: &[String],
    ) {

        /// 缩进
        let dec = 0;

        println!("{:?}",book.nav());
        for ele in book.nav() {
            self.print_nav(0, ele);
        }


    }
}