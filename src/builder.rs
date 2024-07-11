use common::EpubItem;

use crate::{
    zip_writer, EpubAssets, EpubBook, EpubHtml, EpubMetaData, EpubNav, EpubResult, EpubWriter,
};

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

impl Default for EpubBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EpubBuilder {
    pub fn new() -> Self {
        EpubBuilder {
            book: EpubBook::default(),
            custome_nav: false,
            nav: Vec::new(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.book.set_title(title);
        self
    }

    pub fn with_identifier(mut self, identifier: &str) -> Self {
        self.book.set_identifier(identifier);
        self
    }
    pub fn with_creator(mut self, creator: &str) -> Self {
        self.book.set_creator(creator);
        self
    }
    pub fn with_description(mut self, description: &str) -> Self {
        self.book.set_description(description);
        self
    }
    pub fn with_contributor(mut self, contributor: &str) -> Self {
        self.book.set_contributor(contributor);
        self
    }
    pub fn with_date(mut self, date: &str) -> Self {
        self.book.set_date(date);
        self
    }
    pub fn with_format(mut self, format: &str) -> Self {
        self.book.set_format(format);
        self
    }
    pub fn with_publisher(mut self, publisher: &str) -> Self {
        self.book.set_publisher(publisher);
        self
    }
    pub fn with_subject(mut self, subject: &str) -> Self {
        self.book.set_subject(subject);
        self
    }

    pub fn with_last_modify(mut self, last_modify: &str) -> Self {
        self.book.set_last_modify(last_modify);
        self
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
            .add_meta(EpubMetaData::default().with_attr(key, value));
        self
    }

    ///
    /// 添加资源文件
    ///
    pub fn add_assets(mut self, file_name: &str, data: Vec<u8>) -> Self {
        self.book.add_assets(
            EpubAssets::default()
                .with_file_name(file_name)
                .with_data(data),
        );
        self
    }

    ///
    /// 设置封面
    ///
    /// [file_name] epub中的文件名，不是本地文件名
    /// [data] 数据
    ///
    pub fn cover(mut self, file_name: &str, data: Vec<u8>) -> Self {
        self.book.set_cover(
            EpubAssets::default()
                .with_file_name(file_name)
                .with_data(data),
        );

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
        self.book.add_chapter(chapter);
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
            for ele in &mut self.nav {
                self.book.add_nav(ele.clone());
            }
        } else {
            // 生成简单目录
            let mut nav: Vec<EpubNav> = Vec::new();
            for ele in self.book.chapters() {
                // 不能一次循环直接添加，因为会出现重复借用
                nav.push(
                    EpubNav::default()
                        .with_title(ele.title())
                        .with_file_name(ele.file_name()),
                );
            }

            for ele in nav {
                self.book.add_nav(ele);
            }
        }
    }

    fn gen_last_modify(&mut self) {
        if self.book.last_modify().is_none() {
            self.book.set_last_modify(
                format!("{}", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%Z")).as_str(),
            );
        }
    }

    ///
    /// 返回epub实例，将会消耗构造器所有权
    ///
    ///
    pub fn book(mut self) -> EpubBook {
        self.gen_last_modify();
        self.gen_nav();
        self.book
    }

    ///
    /// 输出到文件
    ///
    pub fn file(mut self, file: &str) -> EpubResult<()> {
        self.gen_last_modify();
        self.gen_nav();
        self.book.write(file)
    }

    ///
    /// 输出到内存
    ///
    pub fn mem(mut self) -> EpubResult<Vec<u8>> {
        self.gen_last_modify();
        self.gen_nav();

        let mut writer = zip_writer::ZipMemoeryWriter::new("")?;

        self.book.write_with_writer(&mut writer)?;

        writer.data()
    }
}

#[cfg(test)]
mod tests {
    use crate::EpubHtml;

    use super::EpubBuilder;

    #[test]
    fn test() {
        EpubBuilder::default()
            .with_title("书名")
            .with_creator("作者")
            .with_date("2024-03-14")
            .with_description("一本好书")
            .with_identifier("isbn")
            .with_publisher("行星出版社")
            .add_chapter(
                EpubHtml::default()
                    .with_file_name("0.xml")
                    .with_data("<p>锻炼</p>".to_string().as_bytes().to_vec()),
            )
            .add_assets("1.css", "p{color:red}".to_string().as_bytes().to_vec())
            .metadata("s", "d")
            .metadata("h", "m")
            .file("target/build.epub")
            .unwrap();
    }
}
