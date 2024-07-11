#[derive(Debug)]
pub enum LinkRel {
    CSS,
    OTHER,
}

pub trait EpubItem {
    ///
    /// 文件路径
    ///
    /// 注意，如果是 EPUB 目录下的文件，返回的时候不会带有EPUB路径
    ///
    fn file_name(&self) -> &str;

    ///
    /// 设置文件路径
    ///
    fn set_file_name(&mut self, value: &str);

    fn id(&self) -> &str;
    fn set_id(&mut self, id: &str);

    ///
    ///
    /// 是否是mainifest
    ///
    /// 是代表该文件不会出现在opf中
    ///
    fn is_manifest(&self) -> bool {
        let name = self.file_name();
        if name == "mimetype" {
            return true;
        }
        false
    }

    fn set_data(&mut self, data: Vec<u8>);
    // /
    // / 返回数据
    // /
    // fn data(&mut self) -> Option<&[u8]>;

    // fn read_data(&mut self) -> Option<&[u8]>;
}

pub static EPUB: &str = "EPUB/";
pub static TOC: &str = "EPUB/toc.ncx";
pub static NAV: &str = "EPUB/nav.xhtml";
pub static COVER: &str = "EPUB/cover.xhtml";
pub static OPF: &str = "EPUB/content.opf";

#[derive(Debug)]
pub enum EpubItemType {
    UNKNOWN,
    IMAGE,
    STYLE,
    SCRIPT,
    NAVIGATION,
    VECTOR,
    FONT,
    VIDEO,
    AUDIO,
    DOCUMENT,
    CONVER,
}

impl EpubItemType {
    pub fn code(&self) -> isize {
        match self {
            Self::UNKNOWN => 0,
            Self::IMAGE => 1,
            Self::STYLE => 2,
            Self::SCRIPT => 3,
            Self::NAVIGATION => 4,
            Self::VECTOR => 5,
            Self::FONT => 6,
            Self::VIDEO => 7,
            Self::AUDIO => 8,
            Self::DOCUMENT => 9,
            Self::CONVER => 10,
        }
    }
}

impl std::fmt::Display for EpubItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl std::fmt::Display for LinkRel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::CSS => {
                    "stylesheet"
                }
                Self::OTHER => {
                    "other"
                }
            }
        )
    }
}
