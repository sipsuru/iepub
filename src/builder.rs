use common::EpubItem;

use crate::{zip_writer, EpubAssets, EpubBook, EpubHtml, EpubMetaData, EpubNav, EpubResult, EpubWriter};

///
/// 简化epub构建
///
pub struct EpubBuilder {
    book: EpubBook,

    /// 是否自定义导航
    /// 默认为false
    custome_nav: bool,

    nav: Vec<EpubNav>,
}

impl EpubBuilder {
    pub fn new() -> Self {
        EpubBuilder {
            book: EpubBook::default(),
            custome_nav: false,
            nav: Vec::new(),
        }
    }

    pub fn custome_nav(mut self, value: bool) -> Self {
        self.custome_nav = value;
        self
    }

    ///
    /// 添加 metadata
    ///
    /// 每一对kv都会生成新的meta元素
    ///
    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.book
            .add_meta(EpubMetaData::default().push_attr(key, value));
        self
    }

    ///
    /// 添加资源文件
    ///
    pub fn assets(mut self, file_name: &str, data: Vec<u8>) -> Self {
        self.book
            .assets
            .push(EpubAssets::default().file_name(file_name).data(data));
        self
    }

    ///
    /// 设置封面
    ///
    /// [file_name] epub中的文件名，不是本地文件名
    /// [data] 数据
    ///
    pub fn conver(mut self, file_name: &str, data: Vec<u8>) -> Self {
        self.book.cover = Some(EpubAssets::default().file_name(file_name).data(data));

        self
    }

    ///
    /// 添加文章
    ///
    /// 注意，将会按照文章添加顺序生成一个简易的目录页
    ///
    /// 如果需要更为复杂的自定义目录，需要调用 custome_nav(true) 方法
    ///
    ///
    pub fn add_chapter(mut self, chapter: EpubHtml) -> Self {
        self.book.chapters.push(chapter);
        self
    }

    ///
    /// 添加目录导航
    ///
    pub fn add_nav(mut self, nav: EpubNav) -> Self {
        self.nav.push(nav);
        self
    }

    fn gen_nav(&mut self) {
        if self.custome_nav {
            self.book.nav.append(&mut self.nav);
        } else {
            // 生成简单目录
            let mut nav: Vec<EpubNav> = Vec::new();
            for (index, ele) in self.book.chapters.iter().enumerate() {
                self.book.nav.push(
                    EpubNav::default()
                        .title(ele.get_title())
                        .file_name(ele.get_file_name()),
                );
            }
        }
    }

    ///
    /// 输出到文件
    ///
    pub fn file(mut self, file: &str) -> EpubResult<()> {
        self.gen_nav();
        self.book.write(file)
    }

    pub fn mem(mut self, mut data: Vec<u8>) -> EpubResult<()> {
        self.gen_nav();

        let mut writer = zip_writer::ZipMemoeryWriter::new("")?;

        self.book.write_with_writer(&mut writer)
    }
}
