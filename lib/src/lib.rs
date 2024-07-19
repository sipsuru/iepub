extern crate iepub_derive;
mod common;

pub mod appender;
pub mod builder;

mod core;
mod html;
pub mod reader;
pub mod zip_writer;
#[allow(dead_code)]
pub mod prelude {
    // pub use core::EP
    pub use crate::core::EpubAssets;
    pub use crate::core::EpubBook;
    pub use crate::core::EpubHtml;
    pub use crate::core::EpubLink;
    pub use crate::core::EpubNav;

    pub use crate::core::EpubMetaData;
    pub use crate::core::EpubWriter;

    pub use crate::core::EpubError;

    pub use crate::core::EpubResult;

    pub use crate::common::LinkRel;
}