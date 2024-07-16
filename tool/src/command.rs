use std::{f32::consts::E, fmt::format, io::Write};

use crate::{
    arg::{self, ArgOption, CommandOptionDef, OptionDef, OptionType},
    msg, Command,
};
use iepub::{EpubBook, EpubError, EpubNav};
// 是否覆盖文件
fn is_overiade(global_opts: &[arg::ArgOption], opts: &[arg::ArgOption]) -> bool {
    global_opts
        .iter()
        .find(|s| s.key == "y")
        .map_or(false, |_| true)
        || opts.iter().find(|s| s.key == "y").map_or(false, |_| true)
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
/// 创建一个命令，定死了代码基本结构
macro_rules! create_command {
    // create_command!(结构体名称, "命令名称",{ arg::CommandOptionDef{} }, exec函数, 额外的成员函数 ),如果没有额外的成员函数，最后也需要以逗号结尾，所以最后部分代码应该是: ,);
    ($name:ident, $com:expr, $def:block, $exe:item, $($fun:item),*) => {
        #[derive(Default)]
        pub(crate) struct $name;
        impl $name {
            pub(crate) fn def() -> arg::CommandOptionDef {
                $def
            }

            $($fun)*
        }
        impl Command for $name {
            fn name(&self) -> String {
                $com.to_string()
            }

            $exe

        }
    };
    // 这里可以省掉最后的逗号，以 ); 结尾即可
    ($name:ident,$com:expr,$def:block,$exe:item) => {
        create_command!($name,$com,$def,$exe,);
    };
}
create_command!(
    BookInfoGetter,
    "get-info",
    {
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
    },
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
);

create_command!(
    GetCover,
    "get-cover",
    {
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
    },
    fn exec(
        &self,
        book: &mut EpubBook,
        global_opts: &[ArgOption],
        opts: &[ArgOption],
        args: &[String],
    ) {
        let cover = book.cover().unwrap();
        let is_over = is_overiade(global_opts, opts);
        for path in args {
            if std::path::Path::new(&path).exists()
                && !is_over
                && get_single_input("Override file？(y/n)")
                    .unwrap()
                    .to_lowercase()
                    != "y"
            {
                continue;
            }
            write_file(path, cover.data().unwrap());
        }
    },
);

create_command!(
    NavScanner,
    "nav",
    {
        CommandOptionDef {
            command: "nav".to_string(),
            desc: "导航".to_string(),
            support_args: 0,
            opts: vec![OptionDef::create(
                "s",
                "输出导航对应文件名",
                OptionType::NoParamter,
            )],
        }
    },
    fn exec(
        &self,
        book: &mut EpubBook,
        _global_opts: &[ArgOption],
        opts: &[ArgOption],
        _args: &[String],
    ) {
        let print_href = opts.iter().find(|s| s.key == "s").map_or(false, |_| true);
        for ele in book.nav() {
            self.print_nav(0, ele, print_href);
        }
    },
    fn print_dec(&self, dec: i32) {
        for _ in 0..dec {
            print!(" ");
        }
    },
    fn print_nav(&self, dec: i32, nav: &EpubNav, print_href: bool) {
        self.print_dec(dec);
        println!(
            "{} href=[{}]",
            nav.title(),
            if print_href { nav.file_name() } else { "" }
        );
        for ele in nav.child() {
            self.print_nav(dec + 2, ele, print_href);
        }
    }
);

fn write_file(path: &str, data: &[u8]) {
    let mut fs = std::fs::File::options()
        .truncate(true)
        .create(true)
        .write(true)
        .open(path)
        .unwrap();
    fs.write_all(data).unwrap();
}

create_command!(
    GetImage,
    "get-image",
    {
        arg::CommandOptionDef {
        command: "get-image".to_string(),
        desc: "提取图片".to_string(),
        support_args: 0,
        opts: vec![
            OptionDef::create("d", "输出目录", OptionType::String),
            OptionDef::over(),
            OptionDef::create("p", "文件名前缀，例如-d out -p image,文件将会被写入到 out/image01.jpg，原有文件名将会被忽略", OptionType::String),
        ],
    }
    },
    fn exec(
        &self,
        book: &mut EpubBook,
        global_opts: &[ArgOption],
        opts: &[ArgOption],
        _args: &[String],
    ) {
        let dir_o = opts
            .iter()
            .find(|s| s.key == "d")
            .and_then(|f| f.value.as_ref());
        let is_over = is_overiade(global_opts, opts);

        let prefix = opts
            .iter()
            .find(|s| s.key == "p")
            .and_then(|f| f.value.as_ref());
        let mut file_size = 1;
        if let Some(dir) = dir_o {
            for ele in book.assets_mut() {
                let name = ele.file_name().to_lowercase();
                if name.ends_with(".jpg")
                    || name.ends_with(".jpeg")
                    || name.ends_with(".gif")
                    || name.ends_with(".png")
                    || name.ends_with(".webp")
                    || name.ends_with(".svg")
                {
                    let mut file = format!("{dir}/{}", ele.file_name());
                    if let Some(p) = prefix {
                        // 有前缀
                        file = format!(
                            "{dir}/{p}{}{}",
                            file_size,
                            &name[name.rfind(".").unwrap_or(0)..]
                        );
                        file_size += 1;
                    }
                    let n_dir = &file[0..file.rfind("/").unwrap_or(0)];
                    if !std::path::Path::new(n_dir).exists() {
                        msg!("creating dir {}", n_dir);
                        // 创建目录
                        match std::fs::create_dir_all(n_dir) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("create dir {} fail, because {}", n_dir, e.to_string());
                                continue;
                            }
                        };
                    }

                    // 判断文件是否存在

                    if std::path::Path::new(&file).exists()
                        && !is_over
                        && get_single_input("Override file？(y/n)")
                            .unwrap()
                            .to_lowercase()
                            != "y"
                    {
                        continue;
                    }
                    msg!("writing file to {}", file);
                    // 写入文件
                    write_file(&file, ele.data().unwrap());
                }
            }
        }
    },
);

create_command!(
    GetChapter,
    "get-name",
    {
        arg::CommandOptionDef {
            command: "get-chapter".to_string(),
            desc: "提取章节".to_string(),
            support_args: 0,
            opts: vec![
                OptionDef::create("c", "文件路径，可以从nav命令中获取", OptionType::Array),
                OptionDef::create(
                    "d",
                    "输出目录，没有该参数则直接输出到终端",
                    OptionType::String,
                ),
                OptionDef::over(),
                OptionDef::create(
                    "b",
                    "只输出body部分，否则输出完整的xhtml(可能跟原文有所区别)",
                    OptionType::NoParamter,
                ),
            ],
        }
    },
    fn exec(
        &self,
        book: &mut EpubBook,
        global_opts: &[ArgOption],
        opts: &[ArgOption],
        _args: &[String],
    ) {
        let dir = opts
            .iter()
            .find(|f| f.key == "d")
            .and_then(|f| f.value.as_ref());

        let chaps: Vec<&String> = opts
            .iter()
            .filter(|s| s.key == "c" && s.values.is_some())
            .map(|f| f.values.as_ref().unwrap())
            .flat_map(|f| f)
            .collect();

        let is_over = is_overiade(global_opts, opts);

        let print_body = opts.iter().find(|f| f.key == "b").is_some();

        for ele in chaps {
            if let Some(chap) = book.get_chapter(&ele) {
                if let Some(d) = dir {
                    let mut p_dir: std::path::PathBuf =
                        std::path::Path::new(&d).join(chap.file_name());
                    p_dir.pop(); // 获取在文件所在目录了

                    if !p_dir.exists() {
                        msg!("creating dir {:?}", p_dir);
                        match std::fs::create_dir_all(&p_dir) {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("mkdir {:?} fail, because {}", p_dir, e.to_string());
                                continue;
                            }
                        };
                    }
                    let file = format!("{}/{}", d, chap.file_name());

                    if std::path::Path::new(&file).exists()
                        && !is_over
                        && get_single_input("Override file？(y/n)")
                            .unwrap()
                            .to_lowercase()
                            != "y"
                    {
                        continue;
                    }
                    if print_body {
                        write_file(file.as_str(), chap.data().unwrap())
                    } else {
                        let d = chap.format().unwrap_or("".to_string());
                        write_file(file.as_str(), d.as_bytes());
                    }
                } else {
                    // 直接输出到终端
                    println!(
                        "{}",
                        if print_body {
                            String::from_utf8(chap.data().unwrap().to_vec()).unwrap()
                        } else {
                            chap.format().unwrap_or("".to_string())
                        }
                    );
                }
            } else {
                eprintln!("chap {} not exists", ele);
            }
        }
    },
);
