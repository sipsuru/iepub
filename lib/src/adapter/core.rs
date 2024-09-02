use std::io::{Read, Seek, Write};

use crate::{
    common::IResult,
    mobi::core::MobiAssets,
    prelude::{EpubBook, EpubBuilder, EpubHtml, EpubNav, MobiBook, MobiHtml, MobiNav, MobiReader},
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
    for (index, ele) in nav.iter().enumerate() {
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

/// mobi 转epub
///
/// # Examples
/// ```no_run
/// use iepub::prelude::*;
/// let mut book = std::fs::File::open(std::path::PathBuf::from("example.mobi"))
/// .map_err(|e| IError::Io(e))
/// .and_then(|f| MobiReader::new(f))
/// .and_then(|mut f| f.load())
/// .unwrap();
/// 
/// let mut epub = mobi_to_epub(&mut book).unwrap();
/// epub.write("convert.epub").unwrap();
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

    Ok(builder.book())
}

/// 转换 mobi 的 html 文本，主要是处理其中的img标签，添加src属性
fn convert_mobi_html_data(indent: usize, data: &str, assets: &[MobiAssets]) -> Vec<u8> {
    let mut v = data.to_string();
    let indent = (0..indent).map(|_| "..").collect::<Vec<&str>>().join("/");
    for ele in assets {
        let target = format!(r#"src="{}/{}""#, indent, get_mobi_assets_file_name(ele));
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

#[cfg(test)]
mod tests {
    use crate::{
        common::IError,
        mobi::core::MobiAssets,
        prelude::{MobiBook, MobiReader},
    };

    use super::{convert_mobi_html_data, mobi_to_epub};

    #[test]
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

        epub.write("convert.epub").unwrap();
    }

    #[test]
    fn test_convert_html_img() {
        let data = r#"<h1>插图</h1>
<p height="1em" width="0pt" align="center"><font size="7"><b>第六卷</b></font></p><p height="1em" width="0pt" align="center"><font size="6"><b>插图</b></font></p><p height="1em" width="0pt"> <img recindex="00055" align="baseline" width="1086" height="1526"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129720.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00018" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129721.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00056" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129722.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00019" align="baseline" width="580" height="799"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129723.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00061" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129724.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00024" align="baseline" width="759" height="451"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129725.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00062" align="baseline" width="800" height="600"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129726.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00025" align="baseline" width="600" height="800"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129727.jpg"> </a> </p><p height="3pt" width="0pt"> <img recindex="00063" align="baseline" width="500" height="666"></img><a href="https://pic.wenku8.com/pictures/1/1946/105571/129728.jpg"> </a> </p>"#;

        let mut assets = vec![
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
    }
}
