use std::{fs::File, io::Write};

use crate::{EpubError, EpubResult, EpubWriter};

impl From<zip::result::ZipError> for EpubError {
    fn from(value: zip::result::ZipError) -> Self {
        match value {
            zip::result::ZipError::Io(io) => EpubError::Io(io),
            zip::result::ZipError::InvalidArchive(v) => EpubError::InvalidArchive(v),
            zip::result::ZipError::UnsupportedArchive(v) => EpubError::UnsupportedArchive(v),
            zip::result::ZipError::InvalidPassword => EpubError::InvalidPassword,
            zip::result::ZipError::FileNotFound => EpubError::FileNotFound,
            _ => EpubError::Unknown,
        }
    }
}

impl From<std::io::Error> for EpubError {
    fn from(value: std::io::Error) -> Self {
        EpubError::Io(value)
    }
}
///
/// 写入到文件
///
pub struct ZipFileWriter {
    inner: zip::ZipWriter<File>,
}

impl EpubWriter for ZipFileWriter {
    fn write(&mut self, file: &str, data: &[u8]) -> crate::EpubResult<()> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        self.inner.start_file(file, options)?;
        self.inner.write_all(data)?;
        Ok(())
    }

    fn new(filename: &str) -> crate::EpubResult<Self>
    where
        Self: Sized,
    {
        let path = std::path::Path::new(filename);
        match std::fs::File::create(path) {
            Ok(file) => Ok(ZipFileWriter {
                inner: zip::ZipWriter::new(file),
            }),
            Err(e) => Err(EpubError::Io(e)),
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
    fn new(_file: &str) -> crate::EpubResult<Self>
    where
        Self: Sized,
    {
        let u: Vec<u8> = Vec::new();
        let c = std::io::Cursor::new(u);
        Ok(ZipMemoeryWriter {
            inner: zip::ZipWriter::new(c),
        })
    }

    fn write(&mut self, file: &str, data: &[u8]) -> crate::EpubResult<()> {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        self.inner.start_file(file, options)?;
        self.inner.write_all(data)?;
        Ok(())
    }
}

impl ZipMemoeryWriter {
    pub fn data(self) -> EpubResult<Vec<u8>> {
        let mut w = self.inner.finish()?;

        let mut res = Vec::new();
        res.append(w.get_mut());

        Ok(res)
    }
}
