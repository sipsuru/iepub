use std::{
    io::{Cursor, Read, Seek},
    ops::Deref,
};

use common::EpubItem;
use quick_xml::events::BytesStart;
use zip::read::ZipFile;

use crate::{EpubAssets, EpubBook, EpubError, EpubHtml, EpubMetaData, EpubReaderTrait, EpubResult};

macro_rules! invalid {
    ($x:tt) => {
        Err(crate::EpubError::InvalidArchive($x))
    };
    ($x:expr,$y:expr) => {
        $x.or(Err(crate::EpubError::InvalidArchive($y)))?
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
                println!("start {:?}", e.name());
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
    use quick_xml::reader::Reader;

    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Err(e) => {
                return invalid!("err");
            }
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"meta" => {
                    println!("meta {:?}", e);
                    if parent.len() != 2 || parent[1] != "metadata" {
                        println!("meta = {:?}", parent);
                        return invalid!("not valid opf meta");
                    } else {
                        let meta = create_meta(&e);
                        if let Ok(m) = meta {
                            book.add_meta(m);
                        }
                        parent.push("meta".to_string());
                    }
                }
                b"dc:identifier" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf identifier");
                    } else {
                        parent.push("dc:identifier".to_string());
                    }
                }
                b"dc:title" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf title");
                    } else {
                        parent.push("dc:title".to_string());
                    }
                }
                b"dc:creator" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf creator");
                    } else {
                        parent.push("dc:creator".to_string());
                    }
                }
                b"dc:description" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf description");
                    } else {
                        parent.push("dc:description".to_string());
                    }
                }
                b"dc:format" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf format");
                    } else {
                        parent.push("dc:format".to_string());
                    }
                }
                b"dc:publisher" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf publisher");
                    } else {
                        parent.push("dc:publisher".to_string());
                    }
                }
                b"dc:subject" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf subject");
                    } else {
                        parent.push("dc:subject".to_string());
                    }
                }
                b"dc:contributor" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf contributor");
                    } else {
                        parent.push("dc:contributor".to_string());
                    }
                }
                b"dc:date" => {
                    if parent.len() != 2 || parent[1] != "metadata" {
                        return invalid!("not valid opf date");
                    } else {
                        parent.push("dc:date".to_string());
                    }
                }
                _ => {}
            },
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
                            if let Some(m) = book.get_meta(book.meta_len() - 1) {
                                m.set_text(txt.unescape().unwrap().deref());
                            }
                        }
                        "dc:identifier" => {
                            book.set_identifier(txt.unescape().unwrap().deref());
                        }
                        "dc:title" => {
                            book.set_title(txt.unescape().unwrap().deref());
                        }
                        "dc:creator" => {
                            book.set_creator(txt.unescape().unwrap().deref());
                        }
                        "dc:description" => {
                            book.set_description(txt.unescape().unwrap().deref());
                        }
                        "dc:format" => {
                            book.set_format(txt.unescape().unwrap().deref());
                        }
                        "dc:publisher" => {
                            book.set_publisher(txt.unescape().unwrap().deref());
                        }
                        "dc:subject" => {
                            book.set_subject(txt.unescape().unwrap().deref());
                        }
                        "dc:contributor" => {
                            book.set_contributor(txt.unescape().unwrap().deref());
                        }
                        "dc:date" => {
                            book.set_date(txt.unescape().unwrap().deref());
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"metadata" => {
                    if parent.len() != 2 || parent[0] != "package" {
                        return invalid!("not valid opf metadata end");
                    }
                    break;
                }
                b"meta" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "meta" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:identifier" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:identifier" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:title" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:title" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:creator" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:creator" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:format" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:format" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:publisher" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:publisher" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:subject" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:subject" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:contributor" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:contributor" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:date" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:date" {
                        parent.remove(parent.len() - 1);
                    }
                }
                b"dc:description" => {
                    if !parent.is_empty() && parent[parent.len() - 1] == "dc:description" {
                        parent.remove(parent.len() - 1);
                    }
                }
                _ => {
                    break;
                }
            },
            _ => {}
        }
    }
    println!("新");
    Ok(())
}

fn read_spine_xml(
    reader: &mut quick_xml::reader::Reader<&[u8]>,
    book: &mut EpubBook,
    assets: &mut Vec<EpubAssets>,
) -> EpubResult<()> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
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
                    println!("itemref ");
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
    book: &mut EpubBook,
    assets: &mut Vec<EpubAssets>,
) -> EpubResult<()> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = vec!["package".to_string(), "metadata".to_string()];
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::End(e)) => {
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
    println!("{}", xml);
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    // reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    // 模拟 栈，记录当前的层级
    let mut parent: Vec<String> = Vec::new();
    let mut assets: Vec<EpubAssets> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => {
                break;
            }
            Err(e) => {
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
                    println!("manifest");
                    read_manifest_xml(&mut reader, book, &mut assets)?;
                }
                b"spine" => {
                    println!("spine");
                    read_spine_xml(&mut reader, book, &mut assets)?;
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                _ => {}
            },
            Ok(Event::Text(txt)) => {
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
    {
        // 解析meta，获取部分关键数据
        for i in 0..book.meta_len() {
            if let Some(meta) = book.get_meta(i) {
                println!("meta {:?}", meta);
                for (key, value) in meta.attrs() {
                    println!("meta ele {} {} {} ", key, value, key.as_str() == "property");
                    if key.as_str() == "property"
                        && value == "dcterms:modified"
                        && meta.text().is_some()
                    {
                        last_modify = Some(meta.text().unwrap().to_string());
                        // book.set_last_modify(meta.text().unwrap().to_string().as_str());
                    }
                }
            }
        }
    }

    if let Some(l) = last_modify {
        book.set_last_modify(l.as_str());
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
impl<T: Read + Seek> crate::EpubReaderTrait for EpubReader<T> {
    fn read(&mut self, book: &mut EpubBook) -> EpubResult<()> {
        let mut reader = &mut self.inner;

        {
            // 判断文件格式
            let mut content = read_from_zip!(reader, "mimetype");

            if content != "application/epub+zip" {
                return invalid!("not a epub file");
            }
        }

        {
            let mut content = read_from_zip!(reader, "META-INF/container.xml");

            let opf_path = get_opf_location(content.as_str());

            if let Ok(path) = opf_path {
                let mut opf = read_from_zip!(reader, path.as_str());

                read_opf_xml(opf.as_str(), book)?;
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
    let re:std::rc::Rc<std::cell::RefCell<Box<dyn EpubReaderTrait>>> = std::rc::Rc::new(std::cell::RefCell::new(Box::new(reader)));
    book.reader = Some(std::rc::Rc::clone(&re));

    (*re.borrow_mut()).read(&mut book)?;
    Ok(book)
}

///
/// 从文件读取epub
///
pub fn read_from_file(file: &str) -> EpubResult<EpubBook> {
    let r = zip::ZipArchive::new(std::fs::File::open(file)?)?;
    let mut reader = EpubReader {
        inner: r,
        lazy: true,
    };

    let mut book = EpubBook::default();
    let re:std::rc::Rc<std::cell::RefCell<Box<dyn EpubReaderTrait>>> = std::rc::Rc::new(std::cell::RefCell::new(Box::new(reader)));
    book.reader = Some(std::rc::Rc::clone(&re));

    (*re.borrow_mut()).read(&mut book)?;
    Ok(book)
}

impl<T: Read + Seek> EpubReader<T> {
    pub fn set_lazy(&mut self, lazy: bool) {
        self.lazy = lazy;
    }

    // /
    // / 读取epub中的某个文件
    // /
    // / [file_name] 绝对路径，需要自行判断是否添加EPUB前缀
    // /
    // pub(crate) fn read_file(&mut self, file_name: &str) -> EpubResult<Vec<u8>> {
    //     let mut file = invalid!(self.inner.by_name(file_name), "not exist");
    //     let mut content = Vec::new();
    //     invalid!(file.read_to_end(&mut content), "read err");
    //     Ok(content)
    // }

    // /
    // / 读取epub中的某个文件
    // /
    // / [file_name] 绝对路径，需要自行判断是否添加EPUB前缀
    // /
    // pub(crate) fn read_string(&mut self, file_name: &str) -> EpubResult<String> {
    //     let mut file = invalid!(self.inner.by_name(file_name), "not exist");
    //     let mut content = String::new();
    //     invalid!(file.read_to_string(&mut content), "read err");
    //     Ok(content)
    // }
}

#[cfg(test)]
mod tests {
    use crate::{builder::EpubBuilder, reader::read_from_vec, EpubHtml};

    use super::EpubReader;

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

        let mut data = create_book().mem().unwrap();

        let mut book = read_from_vec(data);
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

        assert_eq!(b.assets.len(), nb.assets.len());
        // 多出来的一个是导航 nav.xhtml
        assert_eq!(b.chapters.len() + 1, nb.chapters.len());

        // 读取html
        let mut chapter = nb.get_chapter("0.xhtml");
        // assert_ne!(None, chapter);
        assert_eq!(true, chapter.is_some());
        let mut c = &mut chapter.unwrap();

        let data = c.data();

        assert_eq!(true, data.is_some());
        let d = String::from_utf8(data.unwrap().to_vec()).unwrap();
        println!("d [{}]", d);

        assert_eq!(
            r"
    <h1>ok</h1>
html
  ",
            d
        );



        for a in nb.assets() {
         println!("ass=[{}]",String::from_utf8(a.data().unwrap().to_vec()).unwrap() );   
        }

        // println!("{}",c.data().map(|f|String::from_utf8(f.to_vec())).unwrap().unwrap());
    }
}
