use std::str::FromStr;
use std::collections::HashMap;

use common::LinkRel;
use derive::epub_base;
use html::{to_html, to_nav_html, to_opf, to_toc_xml};

mod html;
mod zip_writer;

shadow_rs::shadow!(build);
#[warn(dead_code)]
/**
 * 链接文件，可能是css
 */
#[derive(Debug)]
pub struct EpubLink {
    pub rel: LinkRel,
    pub file_type: String,
    pub href: String,
}

#[epub_base]
#[derive(Debug, Default)]
pub struct EpubHtml {
    pub lang: String,
    links: Option<Vec<EpubLink>>,
    /// 章节名称
    title: String,
    /// 自定义的css，会被添加到link下
    css: Option<String>,
}

impl EpubHtml {
    pub fn set_title(&mut self, title: &str) {
        self.title.clear();
        self.title.push_str(title);
    }

    pub fn get_title(&self) -> &str {
        &self.title
    } 

    pub fn set_css(&mut self, css: &str) {
        self.css = Some(String::from(css));
    }

    pub fn get_css(&self) -> Option<&str> {
        self.css.as_deref()
    }

    fn set_language(&mut self, lang: &str) {
        self.lang = String::from_str(lang).unwrap();
    }

    pub fn add_link(&mut self, link: EpubLink) {
        if let Some(links) = &mut self.links {
            links.push(link);
        } else {
            self.links = Some(vec![link]);
        }
    }

    fn get_links(&mut self) -> Option<&mut Vec<EpubLink>> {
        self.links.as_mut()
    }
}

// impl Deref for EpubNcx {
//     type Target = EpubItem;
//     fn deref(&self) -> &Self::Target {
//         &self.item
//      }

// }

// impl DerefMut for EpubNcx {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.item
//     }
// }

///
/// 非章节资源
///
/// 例如css，字体，图片等
///
#[epub_base]
#[derive(Debug, Default)]
pub struct EpubAssets {
    
}

///
/// 目录信息
///
/// 支持嵌套
///
#[epub_base]
#[derive(Debug, Default)]
pub struct EpubNav {
    /// 章节目录
    /// 如果需要序号需要调用方自行处理
    title: String,
    child: Vec<EpubNav>,
}
///
/// 书籍元数据
///
/// 自定义的数据，不在规范内
///
#[derive(Debug, Default)]
pub struct EpubMetaData {
    /// 属性
    attr: HashMap<String, String>,
    /// 文本
    text: Option<String>,
}

impl EpubMetaData {
    pub fn push_attr(mut self, key: &str, value: &str) -> Self {
        self.attr.insert(String::from(key), String::from(value));
        self
    }

    pub fn set_text(mut self, text: &str) -> Self {
        if let Some(t) = &mut self.text {
            t.clear();
            t.push_str(text);
        } else {
            self.text = Some(String::from(text));
        }
        self
    }

    pub fn get_text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn get_attrs(&self) -> std::collections::hash_map::Iter<'_, String, String> {
        self.attr.iter()
    }
}

#[derive(Debug, Default)]
pub struct EpubBookInfo {
    /// 书名
    title: String,

    /// 标志，例如imbi
    identifier: String,
    /// 作者
    creator: Option<String>,
    ///
    /// 简介
    ///
    description: Option<String>,
    /// 捐赠者？
    contributor: Option<String>,

    /// 出版日期
    date: Option<String>,

    /// 格式?
    format: Option<String>,
    /// 出版社
    publisher: Option<String>,
    /// 主题？
    subject: Option<String>,
}

/// 书本
#[derive(Debug, Default)]
pub struct EpubBook {
    /// 上次修改时间
    last_modify: Option<String>,
    /// 书本信息
    info: EpubBookInfo,
    /// 元数据
    meta: Vec<EpubMetaData>,
    /// 目录信息
    nav: Vec<EpubNav>,
    /// 资源
    assets: Vec<EpubAssets>,
    /// 章节
    chapters: Vec<EpubHtml>,
    /// 封面
    cover: Option<EpubAssets>,
}


impl EpubBook {
    pub fn set_title(&mut self, title: &str) {
        self.info.title.clear();
        self.info.title.push_str(title);
    }
    pub fn get_title(&self) -> &str {
        self.info.title.as_str()
    }
    pub fn get_identifier(&self) -> &str {
        self.info.identifier.as_str()
    }
    pub fn set_identifier(&mut self, identifier: &str) {
        self.info.identifier.clear();
        self.info.identifier.push_str(identifier);
    }



    derive::epub_method_option!(creator);
    derive::epub_method_option!(description);
    derive::epub_method_option!(contributor);
    derive::epub_method_option!(date);
    derive::epub_method_option!(format);
    derive::epub_method_option!(publisher);
    derive::epub_method_option!(subject);
}

// 元数据
impl EpubBook {
    ///
    /// 设置epub最后修改时间
    ///
    /// # Examples
    ///
    /// ```
    /// let mut epub = EpubBook::default();
    /// epub.set_last_modify("2024-06-28T08:07:07UTC");
    /// ```
    ///
    pub fn set_last_modify(&mut self, value: &str) {
        if let Some(t) = &mut self.last_modify {
            t.clear();
            t.push_str(value);
        } else {
            self.last_modify = Some(String::from(value));
        }
    }

    pub fn get_last_modify(&self) -> Option<&str> {
        self.last_modify.as_deref()
    }

    ///
    /// 添加元数据
    ///
    /// # Examples
    ///
    /// ```
    /// let mut epub = EpubBook::default();
    /// epub.add_meta(EpubMetaData::default().push_attr("k", "v").set_text("text"));
    /// ```
    ///
    pub fn add_meta(&mut self, meta: EpubMetaData) {
        self.meta.push(meta);
    }
    pub fn get_meta(&self) -> &[EpubMetaData] {
        &self.meta
    }
}

type EpubResult<T> = Result<T, EpubError>;

pub(crate) trait EpubWriter {
    /// 新建
    /// file 输出的epub文件路径
    ///
    fn new(file: &str) -> EpubResult<Self>
    where
        Self: Sized;

    ///
    /// file epub中的文件目录
    /// data 要写入的数据
    ///
    fn write(&mut self, file: &str, data: &[u8]) -> EpubResult<()>;
}

#[derive(Debug)]
pub enum EpubError {

    /// io 错误
    Io(std::io::Error),
    /// invalid Zip archive: {0}
    InvalidArchive(&'static str),

    /// unsupported Zip archive: {0}
    UnsupportedArchive(&'static str),

    /// specified file not found in archive
    FileNotFound,

    /// The password provided is incorrect
    InvalidPassword,
    
    Utf8(std::string::FromUtf8Error),

    Xml(quick_xml::Error),
    Unknown,
}

// impl EpubError {
//     fn new() -> Self {
//         EpubError
//     }
// }
impl std::fmt::Display for EpubError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "epub error")
    }
}

static CONTAINER_XML: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile media-type="application/oebps-package+xml" full-path="{opf}"/>
  </rootfiles>
</container>
"#;

impl EpubBook {
    /// 写入基础的文件
    fn write_base(&self, writer: &mut impl EpubWriter) -> Result<(), EpubError> {
        writer.write(
            "META-INF/container.xml",
            CONTAINER_XML.replace("{opf}", common::OPF).as_bytes(),
        )?;
        writer.write("mimetype", "application/epub+zip".as_bytes())?;

        writer.write(common::OPF, to_opf(self,format!("{}-{}", crate::build::PROJECT_NAME,  crate::build::VERSION).as_str()).as_bytes())?;

        Ok(())
    }

    /// 写入资源文件
    fn write_assets(&self, writer: &mut impl EpubWriter) -> Result<(), EpubError> {
        let m = &self.assets;
        for ele in m {
            if ele.data.is_none() {
                continue;
            }
            writer.write(
                format!("{}{}", common::EPUB, ele.file_name.as_str()).as_str(),
                ele.data.as_ref().unwrap(),
            )?;
        }
        Ok(())
    }

    /// 写入章节文件
    fn write_chapters(&self, writer: &mut impl EpubWriter) -> Result<(), EpubError> {
        let chap = &self.chapters;
        for ele in chap {
            if ele.data.is_none() {
                continue;
            }

            let html = to_html(ele);

            writer.write(
                format!("{}{}", common::EPUB, ele.file_name.as_str()).as_str(),
                html.as_bytes(),
            )?;

            // let mut vue:Vec<u8> = Vec::new();
            // let mut xml = quick_xml::Writer::new(std::io::Cursor::new(vue));
            // use quick_xml::events::*;

            // let mut html = BytesStart::new("html");
            // html.extend_attributes(Attribute::)

            // xml.write_event(Event::Start(html))?;

            // xml.write_event()
        }

        Ok(())
    }
    /// 写入目录
    fn write_nav(&self, writer: &mut impl EpubWriter) -> Result<(), EpubError> {
        // 目录包括两部分，一是自定义的用于书本导航的html，二是epub规范里的toc.ncx文件
        writer.write(
            common::NAV,
            to_nav_html(self.get_title(), &self.nav).as_bytes(),
        )?;
        writer.write(
            common::TOC,
            to_toc_xml(self.get_title(), &self.nav).as_bytes(),
        )?;

        Ok(())
    }

    ///
    /// 生成封面
    ///
    /// 拷贝资源文件以及生成对应的xhtml文件
    ///
    fn write_cover(&self, writer: &mut impl EpubWriter) -> Result<(), EpubError> {
        if let Some(cover) = &self.cover {
            writer.write(
                format!("{}{}", common::EPUB, cover.file_name.as_str()).as_str(),
                cover.data.as_ref().unwrap(),
            )?;

            let mut html = EpubHtml::default();
            html.data = Some(
                format!("<img src=\"{}\" alt=\"Cover\"/>", cover.file_name)
                    .as_bytes()
                    .to_vec(),
            );
            html.title = String::from("Cover");
            writer.write(common::COVER, to_html(&html).as_bytes())?;
        }
        Ok(())
    }

    pub fn write(&self, file: &str) -> Result<(), EpubError> {
        let mut writer = zip_writer::ZipCratesWriter::new(file).expect("create error");

        self.write_base(&mut writer)?;
        self.write_assets(&mut writer)?;
        self.write_chapters(&mut writer)?;
        self.write_nav(&mut writer)?;
        self.write_cover(&mut writer)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{fs::File, io::Read, path::Path};

    use common::EpubItem;

    use crate::{EpubAssets, EpubBook, EpubHtml, EpubNav};

    #[test]
    fn write_assets() {
        let mut book = EpubBook::default();

        // 添加文本资源文件
        let assets = &mut book.assets;

        let mut css = EpubAssets::default();
        css.set_file_name("style/1.css");
        css.data = Some(String::from("ok").as_bytes().to_vec());

        assets.push(css);

        // 添加目录，注意目录和章节并无直接关联关系，需要自行维护保证导航到正确位置
        let mut n = EpubNav::default();
        n.title = String::from("作品说明");
        n.set_file_name("chaps/0.xhtml");

        let mut n1 = EpubNav::default();
        n1.title = String::from("第一卷");

        let mut n2 = EpubNav::default();
        n2.title = String::from("第一卷 第一章");
        n2.set_file_name("chaps/1.xhtml");

        let mut n3 = EpubNav::default();
        n3.title = String::from("第一卷 第二章");
        n3.set_file_name("chaps/2.xhtml");
        n1.child.push(n2);

        book.nav.push(n);
        book.nav.push(n1);
        // 添加章节
        let mut chap = EpubHtml::default();
        chap.set_file_name("chaps/0.xhtml");
        chap.title.push_str("标题1");
        // 章节的数据并不需要填入完整的html，只需要片段即可，输出时会结合其他数据拼接成完整的html
        chap.data = Some(String::from("<p>章节内容html片段</p>").as_bytes().to_vec());

        book.chapters.push(chap);

        chap = EpubHtml::default();
        chap.set_file_name("chaps/1.xhtml");
        chap.title.push_str("标题2");
        chap.data = Some(String::from("第一卷 第一章content").as_bytes().to_vec());

        book.chapters.push(chap);
        chap = EpubHtml::default();
        chap.set_file_name("chaps/2.xhtml");
        chap.title.push_str("标题2");
        chap.data = Some(String::from("第一卷 第二章content").as_bytes().to_vec());

        book.chapters.push(chap);

        book.set_title("书名");
        book.set_creator("作者");
        book.set_identifier("id");
        book.set_description("desc");
        // epub.cover = Some(EpubAssets::default());

        let mut cover = EpubAssets::default();
        cover.set_file_name("cover.jpg");

        let p = Path::new("cover.jpg");
        println!("{:?}", std::env::current_dir());
        let mut cf = File::open(p).unwrap();
        let mut data: Vec<u8> = Vec::new();
        cf.read_to_end(&mut data).unwrap();

        cover.data = Some(data);

        book.cover = Some(cover);

        book.write("target/test.epub").expect("write error");
    }
}
