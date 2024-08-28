use std::{fs::File, io::Write};

use crate::prelude::*;

impl From<zip::result::ZipError> for IError {
    fn from(value: zip::result::ZipError) -> Self {
        match value {
            zip::result::ZipError::Io(io) => IError::Io(io),
            zip::result::ZipError::InvalidArchive(v) => IError::InvalidArchive(v),
            zip::result::ZipError::UnsupportedArchive(v) => IError::UnsupportedArchive(v),
            zip::result::ZipError::InvalidPassword => IError::InvalidPassword,
            zip::result::ZipError::FileNotFound => IError::FileNotFound,
            _ => IError::Unknown,
        }
    }
}


///
/// 写入到文件
///
pub struct ZipFileWriter {
    pub(crate) inner: zip::ZipWriter<File>,
}

impl EpubWriter for ZipFileWriter {
    fn write(&mut self, file: &str, data: &[u8]) -> IResult<()> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        self.inner.start_file(file, options)?;
        self.inner.write_all(data)?;
        Ok(())
    }

    fn new(filename: &str) -> IResult<Self>
    where
        Self: Sized,
    {
        let path = std::path::Path::new(filename);
        // 创建上级目录
        let parent = path.parent();
        if parent.is_some() && parent.map(|f| !f.exists()).unwrap_or(false) {
            std::fs::create_dir_all(parent.unwrap())?;
        }

        match std::fs::File::create(path) {
            Ok(file) => Ok(ZipFileWriter {
                inner: zip::ZipWriter::new(file),
            }),
            Err(e) => Err(IError::Io(e)),
        }
    }
}

///
/// 写入到内存
///
pub struct ZipMemoeryWriter {
    inner: zip::ZipWriter<std::io::Cursor<Vec<u8>>>,
}

impl EpubWriter for ZipMemoeryWriter {
    fn new(_file: &str) -> IResult<Self>
    where
        Self: Sized,
    {
        let u: Vec<u8> = Vec::new();
        let c = std::io::Cursor::new(u);
        Ok(ZipMemoeryWriter {
            inner: zip::ZipWriter::new(c),
        })
    }

    fn write(&mut self, file: &str, data: &[u8]) -> IResult<()> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        self.inner.start_file(file, options)?;
        self.inner.write_all(data)?;
        Ok(())
    }
}

impl ZipMemoeryWriter {
    pub fn data(self) -> IResult<Vec<u8>> {
        let mut w = self.inner.finish()?;

        let mut res = Vec::new();
        res.append(w.get_mut());

        Ok(res)
    }
}
