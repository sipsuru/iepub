/// 章节信息，html片段
#[derive(Debug)]
pub struct MobiHtml {
    title: String,
    /// 原始数据，经编解码后方可阅读
    raw: Option<Vec<u8>>,
    /// 在整个文本中的索引位置
    index: usize,
    /// 可阅读的文本
    data: String,

    nav_id: usize,
}

impl MobiHtml {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn nav_id(&self)->usize{
        self.nav_id
    }
}

#[derive(Debug, Clone)]
pub struct MobiNav {
    /// id，唯一
    pub(crate) id: usize,
    pub(crate) title: String,
    pub(crate) href: usize,
    pub(crate) children: Vec<MobiNav>,
}

impl MobiNav {
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn default(id: usize) -> Self {
        Self {
            id,
            title: Default::default(),
            href: Default::default(),
            children: Default::default(),
        }
    }

    pub fn id(&self)->usize{
        self.id
    }

    pub fn children(&self)->std::slice::Iter<MobiNav>{
        self.children.iter()
    }
}

/// 由于目录存在嵌套，所以需要拿到最底层的那级目录，这样才能准确的拆分文本
///
fn flatten_nav(nav: &[MobiNav]) -> Vec<&MobiNav> {
    let mut n = Vec::new();
    for ele in nav {
        if ele.children.is_empty() {
            n.push(ele);
        } else {
            n.append(&mut flatten_nav(&ele.children));
        }
    }
    n
}
#[derive(Debug)]
pub struct MobiAssets {
    pub(crate) _file_name: String,
    pub(crate) media_type: String,
    pub(crate) _data: Option<Vec<u8>>,
    pub(crate) recindex: usize,
}

impl MobiAssets {
    pub fn data(&self) -> Option<&[u8]> {
        self._data.as_ref().map(|f| f.as_slice())
    }
    pub fn file_name(&self) -> &str {
        &self._file_name
    }
}

#[derive(Debug)]
pub struct MobiBook {
    info: crate::common::BookInfo,
    /// 上次修改时间
    last_modify: Option<String>,
    /// 电子书创建者信息
    generator: Option<String>,
    /// 章节
    chapters: Vec<MobiHtml>,
    /// 封面
    cover: Option<MobiAssets>,
    /// 所有图片
    images: Vec<MobiAssets>,
    /// 目录
    nav: Option< Vec<MobiNav>>,

}

impl MobiBook {
    iepub_derive::option_string_method!(info, creator);
    iepub_derive::option_string_method!(info, description);
    iepub_derive::option_string_method!(info, contributor);
    iepub_derive::option_string_method!(info, date);
    iepub_derive::option_string_method!(info, format);
    iepub_derive::option_string_method!(info, publisher);
    iepub_derive::option_string_method!(info, subject);
    // /
    // / 设置epub最后修改时间
    // /
    // / # Examples
    // /
    // / ```
    // / let mut epub = EpubBook::default();
    // / epub.set_last_modify("2024-06-28T08:07:07UTC");
    // / ```
    // /
    iepub_derive::option_string_method!(last_modify);
    iepub_derive::option_string_method!(generator);

    pub fn set_title(&mut self, title: &str) {
        self.info.title.clear();
        self.info.title.push_str(title);
    }
    pub fn title(&self) -> &str {
        self.info.title.as_str()
    }
    pub fn with_title(mut self, title: &str) -> Self {
        self.set_title(title);
        self
    }
    pub fn identifier(&self) -> &str {
        self.info.identifier.as_str()
    }
    pub fn set_identifier(&mut self, identifier: &str) {
        self.info.identifier.clear();
        self.info.identifier.push_str(identifier);
    }
    pub fn with_identifier(mut self, identifier: &str) -> Self {
        self.set_identifier(identifier);
        self
    }

    pub fn set_cover(&mut self, cover: MobiAssets) {
        self.cover = Some(cover);
    }

    pub fn cover(&self) -> Option<&MobiAssets> {
        self.cover.as_ref()
    }

    pub fn cover_mut(&mut self) -> Option<&mut MobiAssets> {
        self.cover.as_mut()
    }
    pub fn assets_mut(&mut self) -> std::slice::IterMut<MobiAssets> {
        self.images.iter_mut()
    }

    pub fn assets(&self) -> std::slice::Iter<MobiAssets> {
        self.images.iter()
    }

    pub fn chapters_mut(&mut self) -> std::slice::IterMut<MobiHtml> {
        self.chapters.iter_mut()
    }

    pub fn chapters(&self) -> std::slice::Iter<MobiHtml> {
        self.chapters.iter()
    }

    pub fn nav(&self)->Option<std::slice::Iter<MobiNav>>{
        self.nav.as_ref().map(|f|f.iter())
    }

}

use std::io::{Read, Seek};

use crate::common::IResult;

use super::reader::{do_time_format, MobiReader};

impl<T: Read + Seek> MobiReader<T> {
    pub fn load(&mut self) -> IResult<MobiBook> {
        let meta = self.read_meta_data()?;

        let mut chapters = Vec::new();
        let sec = self.load_text()?;

        let nav = self.read_nav_from_text(&sec[..])?;

        // 根据目录拆分文本

        if let Some(n) =&nav {
            chapters.append(
                &mut flatten_nav(n)
                    .iter()
                    .map(|f| (sec.iter().find(|m| m.end > f.href), f))
                    .filter(|s| s.0.is_some())
                    .map(|f| (f.0.unwrap(), f.1))
                    .map(|(sec, nav)| MobiHtml {
                        nav_id: nav.id,
                        index: sec.index,
                        title: nav.title.clone(),
                        raw: None,
                        data: sec.data.clone(),
                    })
                    .collect(),
            );
        }

        let cover = self.read_cover()?;

        let c = meta.contributor.clone();

        Ok(MobiBook {
            info: meta,
            last_modify: Some(do_time_format(self.pdb_header.modify_date)),
            generator: c,
            chapters,
            cover: cover.map(|f| MobiAssets {
                _file_name: f.get_file_name(),
                media_type: String::new(),
                _data: Some(f.0),
                recindex: 0,
            }),
            images: self.read_all_image()?,
            nav:nav
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::mobi::reader::MobiReader;

    #[test]
    fn test_load() {
        let path = std::env::current_dir().unwrap().join("../dan.mobi");
        let mut mobi =
            MobiReader::new(std::fs::File::open(path.to_str().unwrap()).unwrap()).unwrap();

        let book = mobi.load().unwrap();

        println!("{:?}", book.info);

        println!("======");

        println!("{:?}", book.chapters.len());
        println!("======");

        println!("{:?}", book.chapters[0]);

        println!("======");

        println!("{:?}", book.chapters[43]);
        println!("======");

        println!("{:?}", book.images.len());

        println!("======");
        for ele in &book.chapters {
            println!("{} {}",ele.title,ele.nav_id);
        }
        println!("======");
        println!("{:?}",book.nav);

    }
}
