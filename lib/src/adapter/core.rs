use crate::{
    common::{IError, IResult},
    mobi::{builder::MobiBuilder, core::MobiAssets, image::get_attr_value},
    prelude::{EpubBook, EpubBuilder, EpubHtml, EpubNav, MobiBook, MobiHtml, MobiNav},
};

fn to_epub_nav(mobi: &MobiNav, parent: &str) -> EpubNav {
    let mut n = EpubNav::default();
    n = n.with_title(mobi.title());
    n = n.with_file_name(format!("{parent}{}.xhtml", mobi.title).as_str());

    for ele in mobi.children() {
        n.push(to_epub_nav(
            ele,
            format!("{parent}{}/", mobi.title()).as_str(),
        ));
    }

    n
}

/// 找到章节对应的目录
/// 返回多个层级所属目录
fn get_mobi_chapter_nav<'a>(chap: &MobiHtml, nav: &'a [MobiNav]) -> Option<Vec<&'a MobiNav>> {
    for ele in nav {
        if ele.id() == chap.nav_id() {
            return Some(vec![ele]);
        }
        if let Some(mut v) = get_mobi_chapter_nav(chap, ele.children().as_slice()) {
            v.insert(0, ele);
            return Some(v);
        }
    }
    None
}

fn get_mobi_assets_file_name(a: &MobiAssets) -> String {
    format!("image/{}", a.file_name())
}

/// mobi 转 epub
///
/// # Examples
/// ```no_run
/// use iepub::prelude::*;
/// use iepub::prelude::adapter::mobi_to_epub;
///
/// let mut book = std::fs::File::open(std::path::PathBuf::from("example.mobi"))
/// .map_err(|e| IError::Io(e))
/// .and_then(|f| MobiReader::new(f))
/// .and_then(|mut f| f.load())
/// .unwrap();
///
/// let mut epub = mobi_to_epub(&mut book).unwrap();
/// EpubWriter::write_to_mem(&mut epub, true).unwrap();
/// ```
pub fn mobi_to_epub(mobi: &mut MobiBook) -> IResult<EpubBook> {
    let mut builder = EpubBuilder::new();

    // 添加图片
    for ele in mobi.assets_mut() {
        builder = builder.add_assets(
            get_mobi_assets_file_name(ele).as_str(),
            ele.data().unwrap().to_vec(),
        );
    }

    // 添加目录
    if let Some(nav) = mobi.nav() {
        builder = builder.custome_nav(true);

        for n in nav {
            builder = builder.add_nav(to_epub_nav(n, ""));
        }
    }

    let empty = Vec::new();
    let assets = mobi.assets().as_slice();
    // 添加文本
    for chap in mobi.chapters() {
        let nav: Vec<&str> =
            get_mobi_chapter_nav(chap, mobi.nav().unwrap_or(empty.iter()).as_slice())
                .unwrap()
                .iter()
                .map(|f| f.title())
                .collect();

        builder = builder.add_chapter(
            EpubHtml::default()
                .with_title(chap.title())
                .with_file_name(format!("{}.xhtml", nav.join("/")).as_str())
                .with_data(convert_mobi_html_data(nav.len() - 1, chap.data(), assets)),
        );
    }

    // 封面
    if let Some(cover) = mobi.cover() {
        builder = builder.cover(cover.file_name(), cover.data().unwrap().to_vec());
    }
    // 元数据
    builder = builder
        .with_title(mobi.title())
        .with_identifier(mobi.identifier());

    if let Some(v) = mobi.contributor() {
        builder = builder.with_contributor(v);
    }
    if let Some(v) = mobi.creator() {
        builder = builder.with_creator(v);
    }
    if let Some(v) = mobi.description() {
        builder = builder.with_description(v);
    }
    if let Some(v) = mobi.date() {
        builder = builder.with_date(v);
    }
    if let Some(v) = mobi.format() {
        builder = builder.with_format(v);
    }
    if let Some(v) = mobi.last_modify() {
        builder = builder.with_last_modify(v);
    }
    if let Some(v) = mobi.publisher() {
        builder = builder.with_publisher(v);
    }
    if let Some(v) = mobi.subject() {
        builder = builder.with_subject(v);
    }

    builder.book()
}

/// 转换 mobi 的 html 文本，主要是处理其中的img标签，添加src属性
fn convert_mobi_html_data(indent: usize, data: &str, assets: &[MobiAssets]) -> Vec<u8> {
    let mut v = data.to_string();
    let indent = (0..indent).map(|_| "../").collect::<Vec<&str>>().join("");
    for ele in assets {
        let target = format!(
            r#"src="{}{}""#,
            if indent.is_empty() {
                "./"
            } else {
                indent.as_str()
            },
            get_mobi_assets_file_name(ele)
        );
        // 还有层级问题
        // 有可能误伤，但是暂时没有更好的办法
        v = v
            .replace(
                format!("recindex=\"{:05}\"", ele.recindex).as_str(),
                target.as_str(),
            )
            .replace(
                format!("recindex='{:05}'", ele.recindex).as_str(),
                target.as_str(),
            )
            .replace(
                format!("recindex={:05}", ele.recindex).as_str(),
                target.as_str(),
            );
    }

    v.into_bytes()
}

fn epub_nav_to_mobi_nav(
    nav: &[EpubNav],
    start: usize,
    chap: &[(MobiHtml, String)],
) -> Vec<MobiNav> {
    let mut res = Vec::new();
    for ele in nav.iter().enumerate() {
        let mut n = MobiNav::default(ele.0 + start).with_title(ele.1.title());

        // 关联章节
        if let Some(id) = chap
            .iter()
            .find(|(_, file)| file == ele.1.file_name())
            .map(|f| f.0.id)
        {
            n.chap_id = id;
        }

        if ele.1.child().len() > 0 {
            let c = epub_nav_to_mobi_nav(ele.1.child(), ele.0 + start + 1, chap);
            for ele1 in c {
                n.add_child(ele1);
            }
        }
        res.push(n);
    }
    res
}

/// 处理图片地址，把可能存在的相对路径换成绝对路径
fn convert_epub_html_img(html: String, path: &str) -> String {
    let path = crate::path::Path::system(path).pop();
    // .unwrap_or(std::path::Path::new("/"));

    String::from_utf8(generate_text_img_xml(html.as_str(), &path)).unwrap_or_else(|_| String::new())
}

/// 修改xml片段中的img标签的src属性的路径
pub fn generate_text_img_xml(html: &str, current: &crate::path::Path) -> Vec<u8> {
    let mut text = Vec::new();
    let mut index: usize = 0;
    let chars = html.as_bytes();

    let key = b"<img ";

    while index < chars.len() {
        let mut now = chars[index];
        let mut j = 0;
        while j < key.len() {
            if now == key[j] {
                now = chars[index + j + 1];
            } else {
                break;
            }
            j += 1;
        }
        if j == key.len() {
            // 找到 img 标签，接下来查找 src 属性
            text.append(&mut key.to_vec());
            index += j;
            // 查找完后数据被分成三段，第一段 为开头到 src=，第二段是src=到value结束，第三段是value结束到之后
            // 第一段原样添加，第二段如果找到值替换recindex，没找到则原样添加，第三段继续循环

            let att = get_attr_value(&chars[index - 1..], " src=");
            if let Some(v) = att.0 {
                // 有src属性
                let start = att.1;
                for i in index..index + start {
                    text.push(chars[i]);
                }
                index += start;

                let path = String::from_utf8(v).unwrap_or(String::new());

                // 修正路径
                let t = current.join(path.as_str());
                // let n = t
                //     .canonicalize()
                //     .map(|f| f.to_str().map(|v| v.to_string()))
                //     .map_or(None, |f| f)
                //     .unwrap_or_else(|| String::new());
                let len = att.2;

                text.append(&mut format!("src='{}'", t.to_str()).as_bytes().to_vec());

                index += len - 1;

                continue;
            }

            for i in index..(index + att.1) {
                if i < chars.len() {
                    text.push(chars[i]);
                }
            }
            index += att.1;
        } else {
            text.push(chars[index]);
            index += 1;
        }
    }

    text
}

/// epub 转 mobi
///
/// # Examples
/// ```no_run
/// use iepub::prelude::*;
/// use iepub::prelude::adapter::epub_to_mobi;
/// use iepub::prelude::read_from_file;
///
/// let mut epub = read_from_file("example.epub").unwrap();
/// let mut mobi = epub_to_mobi(&mut epub).unwrap();
/// MobiWriter::new(std::fs::File::create("conver.mobi").unwrap())
/// .with_append_title(false)
/// .write(&mobi)
/// .unwrap();
/// ```
pub fn epub_to_mobi(epub: &mut EpubBook) -> IResult<MobiBook> {
    let mut builder = MobiBuilder::new()
        .with_title(epub.title())
        .with_identifier(epub.identifier());

    if let Some(v) = epub.contributor() {
        builder = builder.with_contributor(v);
    }
    if let Some(v) = epub.creator() {
        builder = builder.with_creator(v);
    }
    if let Some(v) = epub.description() {
        builder = builder.with_description(v);
    }
    if let Some(v) = epub.date() {
        builder = builder.with_date(v);
    }
    if let Some(v) = epub.format() {
        builder = builder.with_format(v);
    }
    if let Some(v) = epub.last_modify() {
        builder = builder.with_last_modify(v);
    }
    if let Some(v) = epub.publisher() {
        builder = builder.with_publisher(v);
    }
    if let Some(v) = epub.subject() {
        builder = builder.with_subject(v);
    }

    let chap = epub.chapters_mut();

    let chap_temp: Vec<(MobiHtml, String)> = chap
        .enumerate()
        .map(|(index, html)| {
            (
                MobiHtml::new(index).with_title(html.title()).with_data(
                    html.data()
                        .map(|f| String::from_utf8(f.to_vec()).or_else(|_| Err(IError::Unknown)))
                        .unwrap_or(Err(IError::Unknown))
                        .map(|v| convert_epub_html_img(v, html.file_name()))
                        // .unwrap_or_else(||Err(FromUtf8Error { bytes: Vec::n, error: e }))
                        .unwrap_or(String::new())
                        .as_str(),
                ),
                html.file_name().to_string(),
            )
        })
        .collect();

    let nav = epub_nav_to_mobi_nav(epub.nav(), 0, &chap_temp);

    builder = builder.custome_nav(true);
    for ele in nav {
        builder = builder.add_nav(ele);
    }
    // 静态资源
    for ele in epub.assets_mut() {
        let data = ele.data().ok_or(IError::Unknown)?.to_vec();
        builder = builder.add_assets(ele.file_name(), data);
    }
    // 添加文本
    for (html, _) in chap_temp {
        builder = builder.add_chapter(html);
    }

    if let Some(c) = epub.cover_mut() {
        builder = builder.cover(c.data().ok_or(IError::Unknown)?.to_vec());
    }

    builder.book()
}

// pub fn epub_to_mobi(epub:&mut EpubBook) ->IResult<>

#[cfg(test)]
mod tests {
    use crate::{
        adapter::core::convert_epub_html_img,
        common::IError,
        epub::core::EpubReaderTrait,
        mobi::core::MobiAssets,
        prelude::{read_from_file, EpubBuilder, EpubHtml, EpubWriter, MobiReader, MobiWriter},
    };
    use std::io::Read;

    use super::{convert_mobi_html_data, epub_to_mobi, mobi_to_epub};

    fn download_file(name: &str, url: &str) {
        if std::fs::metadata(name).is_err() {
            // 下载并解压

            let mut zip = tinyget::get(url)
                .send()
                .map(|v| v.as_bytes().to_vec())
                .map_err(|e| IError::InvalidArchive("download fail"))
                .and_then(|f| {
                    zip::ZipArchive::new(std::io::Cursor::new(f))
                        .map_err(|e| IError::InvalidArchive("download fail"))
                })
                .unwrap();
            let mut zip = zip.by_name(name).unwrap();
            let mut v = Vec::new();
            zip.read_to_end(&mut v).unwrap();
            std::fs::write(name, &mut v).unwrap();
        }
    }
    #[test]
    #[ignore = "dan.mobi"]
    fn test_convert() {
        let mut book = std::env::current_dir()
            .ok()
            .map(|f| f.join("../dan.mobi"))
            .map_or(Err(std::io::Error::other("error")), |f| {
                std::fs::File::open(f)
            })
            .map_err(|_| IError::Unknown)
            .and_then(|f| MobiReader::new(f))
            .and_then(|mut f| f.load())
            .unwrap();

        let mut epub = mobi_to_epub(&mut book).unwrap();
        EpubWriter::write_to_mem(&mut epub, true).unwrap();
        // epub.write("convert.epub").unwrap();
    }

    #[test]
    #[cfg(feature = "no_nav")]
    fn test_convert_no_nav() {
        let name = "convert.mobi";
        download_file(
            name,
            "https://github.com/user-attachments/files/18818424/convert.mobi.zip",
        );

        let path = std::env::current_dir().unwrap().join(name);
        let mut mobi =
            MobiReader::new(std::fs::File::open(path.to_str().unwrap()).unwrap()).unwrap();

        let mut book = mobi.load().unwrap();

        assert_eq!(188, book.chapters().len());
        let mut epub = mobi_to_epub(&mut book).unwrap();

        assert_eq!(188, epub.chapters().len());
        assert_eq!(Some("1"), epub.chapters().next().map(|f| f.title()));

        EpubWriter::write_to_mem(&mut epub, false).unwrap();
    }

    #[test]
    fn test_epub_to_mobi() {
        let resp = tinyget::get("https://www.rust-lang.org/static/images/user-logos/yelp.png")
            .send()
            .unwrap();
        let img = resp.as_bytes().to_vec();
        let img2 =tinyget::get("https://blog.rust-lang.org/images/2024-05-17-enabling-rust-lld-on-linux/ripgrep-comparison.png").send().unwrap().as_bytes().to_vec();

        let mut epub = EpubBuilder::default()
            .with_title("书名")
            .with_creator("作者")
            .with_date("2024-03-14")
            .with_description("一本好书")
            .with_identifier("isbn")
            .with_publisher("行星出版社")
            .add_chapter(
                EpubHtml::default()
                    .with_title("测试标题")
                    .with_file_name("1/0.xhtml")
                    .with_data(
                        "<p>锻炼</p><img src='../1.jpg'/>"
                            .to_string()
                            .as_bytes()
                            .to_vec(),
                    ),
            )
            .add_assets("1.jpg", img.clone())
            .cover("cover.jpg", img2.clone())
            .book()
            .unwrap();

        let mobi = epub_to_mobi(&mut epub).unwrap();
        let mut v = std::io::Cursor::new(Vec::new());
        MobiWriter::new(&mut v)
            .with_append_title(false)
            .write(&mobi)
            .unwrap();

        let n_mobi = MobiReader::new(&mut v).unwrap().load().unwrap();

        assert_eq!(epub.title(), n_mobi.title());
        assert_eq!(epub.description(), n_mobi.description());

        assert_eq!(epub.chapters().len(), mobi.chapters().len());
        assert_eq!(epub.chapters().len(), n_mobi.chapters().len());
        assert_eq!(epub.assets().len(), n_mobi.assets().len());
    }

    #[test]
    fn test_convert_html_img() {
        let data = r#"<h1>插图</h1>
<p height="1em" width="0pt" align="center"><font size="7"><b>第六卷</b></font></p><p height="1em" width="0pt" align="center"><font size="6"><b>插图</b></font></p><p height="1em" width="0pt"> <img recindex="00055" align="baseline" width="1086" height="1526"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129720.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00018" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129721.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00056" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129722.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00019" align="baseline" width="580" height="799"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129723.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00061" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129724.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00024" align="baseline" width="759" height="451"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129725.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00062" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129726.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00025" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129727.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00063" align="baseline" width="500" height="666"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129728.jpg"> </a> </p>"#;

        let assets = vec![
            MobiAssets {
                _file_name: "1.jpg".to_string(),
                media_type: String::new(),
                _data: None,
                recindex: 55,
            },
            MobiAssets {
                _file_name: "2.jpg".to_string(),
                media_type: String::new(),
                _data: None,
                recindex: 56,
            },
        ];

        let r = convert_mobi_html_data(2, data, &assets);

        assert_eq!(
            String::from_utf8(r).unwrap(),
            r#"<h1>插图</h1>
<p height="1em" width="0pt" align="center"><font size="7"><b>第六卷</b></font></p><p height="1em" width="0pt" align="center"><font size="6"><b>插图</b></font></p><p height="1em" width="0pt"> <img src="../../image/1.jpg" align="baseline" width="1086" height="1526"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129720.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00018" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129721.jpg"> </a> </p><p height="3pt" width="0pt"> <img src="../../image/2.jpg" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129722.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00019" align="baseline" width="580" height="799"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129723.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00061" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129724.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00024" align="baseline" width="759" height="451"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129725.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00062" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129726.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00025" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129727.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00063" align="baseline" width="500" height="666"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129728.jpg"> </a> </p>"#
        );

        let r = convert_mobi_html_data(0, data, &assets);

        assert_eq!(
            String::from_utf8(r).unwrap(),
            r#"<h1>插图</h1>
<p height="1em" width="0pt" align="center"><font size="7"><b>第六卷</b></font></p><p height="1em" width="0pt" align="center"><font size="6"><b>插图</b></font></p><p height="1em" width="0pt"> <img src="./image/1.jpg" align="baseline" width="1086" height="1526"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129720.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00018" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129721.jpg"> </a> </p><p height="3pt" width="0pt"> <img src="./image/2.jpg" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129722.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00019" align="baseline" width="580" height="799"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129723.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00061" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129724.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00024" align="baseline" width="759" height="451"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129725.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00062" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129726.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00025" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129727.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00063" align="baseline" width="500" height="666"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129728.jpg"> </a> </p>"#
        );
    }

    #[test]
    fn test_convert_epub_html_img() {
        // let html = r#"<img src="../ok.jpg"/>"#;
        // let v = convert_epub_html_img(html.to_string(), "/parent1/parent.xhtml");

        // println!("{}", v);

        // let html = r#"<img data-src="3.jpg" src="../ok.jpg"/>"#;
        // let v = convert_epub_html_img(html.to_string(), "/parent1/parent.xhtml");

        // println!("{}", v);

        // let html = r#" class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86275.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86275.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86276.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86276.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86277.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86277.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86278.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86278.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86279.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86279.jpg"/>"#;
        //         let v = get_attr_value(html.as_bytes().to_vec().as_slice(), " src=");

        //         assert_ne!(None,v.0);
        //         assert_eq!(91,v.1);

        let html = r#"<img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86275.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86275.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86276.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86276.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86277.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86277.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86278.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86278.jpg"/><img class="imagecontent lazyload" data-src='https://img3.readpai.com/2/2356/121744/86279.jpg' src="../temp/2356/images/www.bilinovel.com/2356/0/86279.jpg"/>
</div>
  </body></html>"#;
        let v = convert_epub_html_img(html.to_string(), "/parent1/parent.xhtml");

        println!("{}", v);
    }
}
