use std::{
    io::{Read, Seek},
    ops::Deref,
};

use quick_xml::events::BytesStart;

use crate::{core::EpubReaderTrait, prelude::*};

macro_rules! invalid {
    ($x:tt) => {
        Err(EpubError::InvalidArchive($x))
    };
    ($x:expr,$y:expr) => {
        $x.or(Err(EpubError::InvalidArchive($y)))?
    };
}

macro_rules! read_from_zip {
    ($m:ident,$x:expr) => {{
        // 读取 container.xml
        let mut file = invalid!($m.by_name($x), stringify!($x not exist));
        let mut content = String::new();
        invalid!(file.read_to_string(&mut content), "read err");
        content
    }};
}

fn get_opf_location(xml: &str) -> EpubResult<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    // reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut res: Result<String, EpubError> = Err(EpubError::InvalidArchive("has no opf"));
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                res = Ok(String::new());
                break;
            }
            Err(e) => {
                res = Err(EpubError::Xml(e));
                break;
            }
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"rootfile" {
                    if let Ok(path) = e.try_get_attribute("full-path") {
                        res = match path.map(|f| {
                            f.unescape_value()
                                .map_or_else(|_| String::new(), |v| v.to_string())
                        }) {
                            Some(v) => Ok(v),
                            None => Err(EpubError::InvalidArchive("has no opf")),
                        };
                        break;
                    }
                }
            }
            _ => (),
        }
        buf.clear();
    }
    res
}

fn create_meta(xml: &BytesStart) -> EpubResult<EpubMetaData> {
    let mut meta = EpubMetaData::default();

    for ele in xml.attributes() {
        if let Ok(a) = ele {
            meta.push_attr(
                String::from_utf8(a.key.0.to_vec()).unwrap().as_str(),
                String::from_utf8(a.value.to_vec()).unwrap().as_str(),
            );
        }
    }
    Ok(meta)
}

fn read_meta_xml(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    book: &mut EpubBook,
) -> EpubResult<()> {
    use quick_xml::events::Event;

    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Err(_e) => {
                return invalid!("err");
            }
            Ok(Event::Start(e)) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())
                    .map_err(EpubError::Utf8)?;

                if name == "meta" {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf meta");
                    } else {
                        let meta = create_meta(&e);
                        if let Ok(m) = meta {
                            book.add_meta(m);
                        }
                        parent.push("meta".to_string());
                    }
                } else if parent.len() != 2 || parent[1] != "metadata" {
                    return invalid!("not valid opf identifier");
                } else {
                    parent.push(name);
                }
            }
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                b"meta" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf meta empty");
                    } else {
                        let meta = create_meta(&e);
                        if let Ok(m) = meta {
                            book.add_meta(m);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Text(txt)) => {
                if !parent.is_empty() {
                    match parent[parent.len() - 1].as_str() {
                        "meta" => {
                            if let Some(m) = book.get_meta_mut(book.meta_len() - 1) {
                                m.set_text(txt.unescape()?.deref());
                            }
                        }
                        "dc:identifier" => {
                            book.set_identifier(txt.unescape()?.deref());
                        }
                        "dc:title" => {
                            book.set_title(txt.unescape()?.deref());
                        }
                        "dc:creator" => {
                            book.set_creator(txt.unescape()?.deref());
                        }
                        "dc:description" => {
                            book.set_description(txt.unescape()?.deref());
                        }
                        "dc:format" => {
                            book.set_format(txt.unescape()?.deref());
                        }
                        "dc:publisher" => {
                            book.set_publisher(txt.unescape()?.deref());
                        }
                        "dc:subject" => {
                            book.set_subject(txt.unescape()?.deref());
                        }
                        "dc:contributor" => {
                            book.set_contributor(txt.unescape()?.deref());
                        }
                        "dc:date" => {
                            book.set_date(txt.unescape()?.deref());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())
                    .map_err(EpubError::Utf8)?;

                if name == "metadata" {
                    if parent.len() != 2 || parent[0] != "package" {
                        return invalid!("not valid opf metadata end");
                    }
                    break;
                }

                if !parent.is_empty() && parent[parent.len() - 1] == name {
                    parent.remove(parent.len() - 1);
                }

                // println!("end {}",String::from_utf8(e.name().as_ref().to_vec()).unwrap());
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_spine_xml(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    book: &mut EpubBook,
    assets: &mut Vec<EpubAssets>,
) -> EpubResult<()> {
    use quick_xml::events::Event;

    // 模拟 栈，记录当前的层级
    let _parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"spine" => {
                        // 写入书本
                        for ele in assets.iter() {
                            if ele.id() == "toc" {
                                continue;
                            }

                            book.add_assets(ele.clone());
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::Empty(e)) => match e.name().as_ref() {
                b"itemref" => {
                    if let Ok(href) = e.try_get_attribute("idref") {
                        if let Some(h) = href.map(|f| {
                            f.unescape_value()
                                .map_or_else(|_| String::new(), |v| v.to_string())
                        }) {
                            let xhtml = assets
                                .iter()
                                .enumerate()
                                .find(|(_index, s)| s.id() == h.as_str());
                            if let Some((index, xh)) = xhtml {
                                book.add_chapter(
                                    EpubHtml::default().with_file_name(xh.file_name()),
                                );
                                assets.remove(index);
                            }
                        }
                    }
                }
                _ => {}
            },

            _ => {
                break;
            }
        }
    }

    Ok(())
}
///
/// 获取所有文件信息
///
fn read_manifest_xml(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    _book: &mut EpubBook,
    assets: &mut Vec<EpubAssets>,
) -> EpubResult<()> {
    use quick_xml::events::Event;

    // 模拟 栈，记录当前的层级
    let _parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::End(_e)) => {
                break;
            }
            Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    b"item" => {
                        let mut a = EpubAssets::default();
                        if let Ok(href) = e.try_get_attribute("href") {
                            if let Some(h) = href.map(|f| {
                                f.unescape_value()
                                    .map_or_else(|_| String::new(), |v| v.to_string())
                            }) {
                                a.set_file_name(h.as_str());
                            }
                        }
                        if let Ok(href) = e.try_get_attribute("id") {
                            if let Some(h) = href.map(|f| {
                                f.unescape_value()
                                    .map_or_else(|_| String::new(), |v| v.to_string())
                            }) {
                                a.set_id(h.as_str());
                            }
                        }

                        assets.push(a);
                    }
                    _ => {
                        // invalid!("item err")
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_opf_xml(xml: &str, book: &mut EpubBook) -> EpubResult<()> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.trim_text(true);

    let mut buf = Vec::new();
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = Vec::new();
    let mut assets: Vec<EpubAssets> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Err(_e) => {
                return invalid!("err");
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"package" => {
                    parent.push("package".to_string());
                }
                b"metadata" => {
                    if parent.len() != 1 || parent[0] != "package" {
                        return invalid!("not valid opf metadata");
                    } else {
                        parent.push("metadata".to_string());
                    }
                    read_meta_xml(&mut reader, book)?;
                }
                b"manifest" => {
                    read_manifest_xml(&mut reader, book, &mut assets)?;
                }
                b"spine" => {
                    read_spine_xml(&mut reader, book, &mut assets)?;
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                _ => {}
            },
            Ok(Event::Text(_txt)) => {
                if !parent.is_empty() {
                    match parent[parent.len() - 1].as_str() {
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"package" => {
                        if parent.len() == 1 && parent[0] == "package" {
                        } else {
                            // xml错误
                        }
                        // parent.remove(to_string());
                    }

                    _ => {}
                }
            }
            _ => {}
        }
        buf.clear();
    }

    let mut last_modify = None;
    let mut cover = None;
    {
        // 解析meta，获取部分关键数据
        for i in 0..book.meta_len() {
            if let Some(meta) = book.get_meta(i) {
                {
                    if let Some(value) = meta.get_attr("name") {
                        if value == "cover" {
                            // 可能是封面
                            if let Some(content) = meta.get_attr("content") {
                                // 对应的封面的id

                                // 查找封面
                                for ele in book.assets() {
                                    if ele.id() == content {
                                        // 封面
                                        cover = Some(ele.clone());
                                        // book.set_cover(ele.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                {
                    if let Some(pro) = meta.get_attr("property") {
                        if pro == "dcterms:modified" && meta.text().is_some() {
                            last_modify = Some(meta.text().unwrap().to_string());
                        }
                    }
                }

                // for (key, value) in meta.attrs() {
                //     if key.as_str() == "property"
                //         && value == "dcterms:modified"
                //         && meta.text().is_some()
                //     {
                //         last_modify = Some(meta.text().unwrap().to_string());
                //         // book.set_last_modify(meta.text().unwrap().to_string().as_str());
                //     }
                // }
            }
        }
    }

    if let Some(l) = last_modify {
        book.set_last_modify(l.as_str());
    }
    if let Some(co) = cover {
        book.set_cover(co);
    }

    Ok(())
}
fn read_nav_point_xml(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    nav: &mut EpubNav,
) -> EpubResult<()> {
    use quick_xml::events::Event;
    

    let mut buf = Vec::new();
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"navLabel" => {
                    parent.push("navLavel".to_string());
                }
                b"text" => {
                    parent.push("text".to_string());
                }
                b"content" => {
                    if let Ok(src) = e.try_get_attribute("src") {
                        if let Some(h) = src.map(|f| {
                            f.unescape_value()
                                .map_or_else(|_| String::new(), |v| v.to_string())
                        }) {
                            nav.set_file_name(&h);
                        }
                    }
                }
                b"navPoint" => {
                    // 套娃了
                    let mut n = EpubNav::default();
                    read_nav_point_xml(reader, &mut n)?;
                    nav.push(n);
                }
                _ => {}
            },
            Ok(Event::End(e)) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())
                    .map_err(EpubError::Utf8)?;

                if name == "navPoint" {
                    break;
                }

                if !parent.is_empty() && parent[parent.len() - 1] == name {
                    parent.remove(parent.len() - 1);
                }
            }
            Ok(Event::Text(e)) => {
                if parent[parent.len() - 1] == "text" {
                    nav.set_title(e.unescape()?.deref());
                }
            }
            Err(_e) => {
                return invalid!("err");
            }
            _ => {}
        }
    }
    Ok(())
}

fn read_nav_xml(xml: &str, book: &mut EpubBook) -> EpubResult<()> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let config = reader.config_mut();
    config.trim_text(true);
    config.expand_empty_elements = true;

    let mut buf = Vec::new();
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = Vec::new();
    let mut assets: Vec<EpubNav> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Err(_e) => {
                return invalid!("err");
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"ncx" => {
                    parent.push("ncx".to_string());
                }
                b"navMap" => {
                    if parent.len() != 1 || parent[0] != "ncx" {
                        return invalid!("err nav 1");
                    }
                    parent.push("navMap".to_string());
                }
                b"navPoint" => {
                    if parent.len() != 2 || parent[1] != "navMap" {
                        return invalid!("err nav 2");
                    }
                    let mut nav = EpubNav::default();
                    read_nav_point_xml(&mut reader, &mut nav)?;
                    assets.push(nav);
                }
                _ => {}
            },
            Ok(Event::End(e)) => {
                let name = String::from_utf8(e.name().as_ref().to_vec())
                    .map_err(EpubError::Utf8)?;

                if name == "navMap" {
                    break;
                }

                if !parent.is_empty() && parent[parent.len() - 1] == name {
                    parent.remove(parent.len() - 1);
                }
            }
            _ => {}
        }
    }
    for ele in assets {
        book.add_nav(ele);
    }
    Ok(())
}
#[derive(Debug, Clone)]
struct EpubReader<T> {
    /// 是否懒加载
    lazy: bool,
    inner: zip::ZipArchive<T>,
}

// impl <T: Read + Seek> From<Vec<u8>> for EpubReader<T> {
//     fn from(value: Vec<u8>) -> Self {

//         EpubReader{
//             inner:
//         }

//     }
// }
impl<T: Read + Seek> EpubReaderTrait for EpubReader<T> {
    fn read(&mut self, book: &mut EpubBook) -> EpubResult<()> {
        let reader = &mut self.inner;

        {
            // 判断文件格式
            let content = read_from_zip!(reader, "mimetype");

            if content != "application/epub+zip" {
                return invalid!("not a epub file");
            }
        }

        {
            let content = read_from_zip!(reader, "META-INF/container.xml");

            let opf_path = get_opf_location(content.as_str());

            if let Ok(path) = opf_path {
                let opf = read_from_zip!(reader, path.as_str());

                read_opf_xml(opf.as_str(), book)?;
            }
        }
        {
            // 读取导航
            if reader.by_name("EPUB/toc.ncx").is_ok() {
                let content = read_from_zip!(reader, "EPUB/toc.ncx");

                read_nav_xml(content.as_str(), book)?;
            }
        }
        Ok(())
    }

    fn read_file(&mut self, file_name: &str) -> EpubResult<Vec<u8>> {
        let mut file = invalid!(self.inner.by_name(file_name), "not exist");
        let mut content = Vec::new();
        invalid!(file.read_to_end(&mut content), "read err");
        Ok(content)
    }

    fn read_string(&mut self, file_name: &str) -> EpubResult<String> {
        let mut file = invalid!(self.inner.by_name(file_name), "not exist");
        let mut content = String::new();
        invalid!(file.read_to_string(&mut content), "read err");
        Ok(content)
    }
}

///
/// 从内存读取epub
///
pub fn read_from_vec(data: Vec<u8>) -> EpubResult<EpubBook> {
    let r = zip::ZipArchive::new(std::io::Cursor::new(data))?;
    let reader = EpubReader {
        inner: r,
        lazy: true,
    };
    let mut book = EpubBook::default();
    let re: std::rc::Rc<std::cell::RefCell<Box<dyn EpubReaderTrait>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Box::new(reader)));
    book.set_reader(std::rc::Rc::clone(&re));

    (*re.borrow_mut()).read(&mut book)?;
    Ok(book)
}

///
/// 从文件读取epub
///
pub fn read_from_file(file: &str) -> EpubResult<EpubBook> {
    let r = zip::ZipArchive::new(std::fs::File::open(file)?)?;
    let reader = EpubReader {
        inner: r,
        lazy: true,
    };

    let mut book = EpubBook::default();
    let re: std::rc::Rc<std::cell::RefCell<Box<dyn EpubReaderTrait>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Box::new(reader)));
    book.set_reader(std::rc::Rc::clone(&re));

    (*re.borrow_mut()).read(&mut book)?;
    Ok(book)
}

impl<T: Read + Seek> EpubReader<T> {
    pub fn set_lazy(&mut self, lazy: bool) {
        self.lazy = lazy;
    }
}

#[cfg(test)]
mod tests {
    use super::read_from_file;
    use crate::prelude::*;
    use crate::{builder::EpubBuilder, reader::read_from_vec};

    #[test]
    fn test() {
        read_from_file("/app/魔女之旅.epub").unwrap();
    }

    #[test]
    fn test_reader() {
        #[inline]
        fn create_book() -> EpubBuilder {
            EpubBuilder::new()
                .with_title("书名")
                .with_format("f书名")
                .with_creator("作者")
                .with_date("2024-03-14")
                .with_description("一本好书")
                .with_identifier("isbn")
                .with_publisher("行星出版社")
                .with_last_modify("last_modify")
                .add_assets("style.css", "ok".as_bytes().to_vec())
                .add_chapter(
                    EpubHtml::default()
                        .with_title("ok")
                        .with_file_name("0.xhtml")
                        .with_data("html".as_bytes().to_vec()),
                )
        }

        let data = create_book().mem().unwrap();

        let book = read_from_vec(data);
        let mut nb = book.unwrap();
        println!("\n{}", nb);
        let b = create_book().book();
        assert_eq!(b.title(), nb.title());
        assert_eq!(b.date(), nb.date());
        assert_eq!(b.creator(), nb.creator());
        assert_eq!(b.description(), nb.description());
        assert_eq!(b.contributor(), nb.contributor());
        assert_eq!(b.format(), nb.format());
        assert_eq!(b.subject(), nb.subject());
        assert_eq!(b.publisher(), nb.publisher());
        assert_eq!(b.identifier(), nb.identifier());
        assert_eq!(b.last_modify(), nb.last_modify());

        assert_eq!(b.assets().len(), nb.assets().len());
        // 多出来的一个是导航 nav.xhtml
        assert_eq!(b.chapters().len() + 1, nb.chapters().len());

        // 读取html
        let chapter = nb.get_chapter("0.xhtml");
        // assert_ne!(None, chapter);
        assert!(chapter.is_some());
        let c = &mut chapter.unwrap();

        let data = c.data();

        assert!(data.is_some());
        let d = String::from_utf8(data.unwrap().to_vec()).unwrap();
        println!("d [{}]", d);

        assert_eq!(
            r"
    <h1>ok</h1>
html
  ",
            d
        );

        for a in nb.assets_mut() {
            println!(
                "ass=[{}]",
                String::from_utf8(a.data().unwrap().to_vec()).unwrap()
            );
        }

        println!("nav ={:?}", nb.nav());
        assert_eq!(1, nb.nav().len());
        assert_eq!("0.xhtml", nb.nav()[0].file_name());

        // println!("{}",c.data().map(|f|String::from_utf8(f.to_vec())).unwrap().unwrap());
    }
}
