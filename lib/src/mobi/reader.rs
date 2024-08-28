//!
//! 支持mobi格式。
//!
//! mobi是采用[PDB](https://wiki.mobileread.com/wiki/PDB#Intro_to_the_Database_format)格式封装，就像epub使用zip格式封装一样
//!

use std::{
    collections::HashMap,
    fmt::format,
    io::{BufRead, BufReader, Error, Read, Seek, SeekFrom},
    ops::Deref,
    os::linux::raw::stat,
    str::Utf8Error,
};

use crate::{
    common::{BookInfo, IError, IResult},
    mobi::core::MobiNav,
};

use super::{
    core::MobiAssets,
    image::{get_suffix, read_image_recindex_from_html, Cover},
    nav::{read_guide_filepos, read_nav_xml},
};

fn vec_u8_to_u64(v: &[u8]) -> u64 {
    let mut u64: u64 = 0;
    for ele in v {
        u64 = u64 << 8;
        u64 = u64 | (*ele as u64)
    }
    u64
}

/// pdb中的一个record info
/// 一组8个字节
#[derive(Default, Debug)]
pub(crate) struct PDBRecordInfo {
    ///  the offset of record n from the start of the PDB of this record
    offset: u32,
    /// bit field. The least significant four bits are used to represent the category values. These are the categories used to split the databases for viewing on the screen. A few of the 16 categories are pre-defined but the user can add their own. There is an undefined category for use if the user or programmer hasn't set this.
    /// 0x10 (16 decimal) Secret record bit.
    /// 0x20 (32 decimal) Record in use (busy bit).
    /// 0x40 (64 decimal) Dirty record bit.
    /// 0x80 (128, unsigned decimal) Delete record on next HotSync.
    attribute: u8,
    /// The unique ID for this record. Often just a sequential count from 0
    /// 实际是只有3个字节，最高位的一个字节不使用
    unique_id: u32,
}

#[derive(Default, Debug)]
pub(crate) struct PDBHeader {
    // name(32)
    name: [u8; 32],
    // attribute(2)
    ///
    /// 0x0002 Read-Only
    /// 0x0004 Dirty AppInfoArea
    /// 0x0008 Backup this database (i.e. no conduit exists)
    /// 0x0010 (16 decimal) Okay to install newer over existing copy, if present on PalmPilot
    /// 0x0020 (32 decimal) Force the PalmPilot to reset after this database is installed
    /// 0x0040 (64 decimal) Don't allow copy of file to be beamed to other Pilot.
    ///
    attribute: u16,
    /// file version
    version: u16,
    /// No. of seconds since start of January 1, 1904.
    ///
    /// [https://wiki.mobileread.com/wiki/PDB#PDB%20Times] 对于时间又有新的规定
    ///
    /// If the time has the top bit set, it's an unsigned 32-bit number counting from 1st Jan 1904
    ///
    /// If the time has the top bit clear, it's a signed 32-bit number counting from 1st Jan 1970.
    ///
    createion_date: u32,
    /// No. of seconds since start of January 1, 1904.
    pub(crate) modify_date: u32,
    /// No. of seconds since start of January 1, 1904.
    last_backup_date: u32,
    /// No. of seconds since start of January 1, 1904.
    modification_number: u32,
    /// offset to start of Application Info (if present) or null
    app_info_id: u32,
    /// offset to start of Sort Info (if present) or null
    sort_info_id: u32,
    /// See above table. (For Applications this data will be 'appl')
    _type: [u8; 4],
    /// See above table. This program will be launched if the file is tapped
    creator: [u8; 4],
    /// used internally to identify record
    unique_id_seed: u32,
    /// Only used when in-memory on Palm OS. Always set to zero in stored files.
    next_record_list_id: u32,
    /// number of records in the file - N
    number_of_records: u16,
    /// record，每个8个字节，所有list结束后，有两个字节的空隙，无实际意义
    record_info_list: Vec<PDBRecordInfo>,
}

impl PDBHeader {
    fn load<T>(reader: &mut T) -> IResult<Self>
    where
        T: Read + Seek,
    {
        let mut header = PDBHeader::default();
        // header.name = reader.read_string(32)?;

        reader.read_exact(&mut header.name)?;

        header.attribute = reader.read_u16()?;
        header.version = reader.read_u16()?;
        header.createion_date = reader.read_u32()?;
        header.modify_date = reader.read_u32()?;
        header.last_backup_date = reader.read_u32()?;
        header.modification_number = reader.read_u32()?;
        header.app_info_id = reader.read_u32()?;
        header.sort_info_id = reader.read_u32()?;

        if "BOOKMOBI" != reader.read_string(8)? {
            return Err(IError::UnsupportedArchive("not a mobi file"));
        }

        // header._type = reader.read_u32()?;
        // header.creator = reader.read_u32()?;
        header.unique_id_seed = reader.read_u32()?;
        header.next_record_list_id = reader.read_u32()?;
        header.number_of_records = reader.read_u16()?;

        let mut record_info_list: Vec<PDBRecordInfo> = vec![];
        // 读取header
        for _ in 0..header.number_of_records {
            let mut info = PDBRecordInfo::default();
            info.offset = reader.read_u32()?;

            let a = reader.read_u32()?;
            // 取最高位的
            info.attribute = (a >> 24) as u8;
            // 去掉最高8位
            info.unique_id = a & 0x00ffffff;
            record_info_list.push(info);
        }
        header.record_info_list = record_info_list;
        Ok(header)
    }
}

#[derive(Default, Debug)]
pub(crate) struct MOBIDOCHeader {
    ///  1 == no compression, 2 = PalmDOC compression, 17480 = HUFF/CDIC compression
    /// 之后跳过2字节无用
    compression: u16,
    /// Uncompressed length of the entire text of the book
    length: u32,
    /// Number of PDB records used for the text of the book.
    record_count: u16,
    /// Maximum size of each record containing text, always 4096
    record_size: u16,
    /// Current reading position, as an offset into the uncompressed text
    /// 如果 compression = 17480  ，这个字段会被拆分开
    position: u32,
    /// compression = 17480 时才有该字段
    ///   0 == no encryption, 1 = Old Mobipocket Encryption, 2 = Mobipocket Encryption
    encrypt_type: u16,
}

impl MOBIDOCHeader {
    fn load<T>(reader: &mut T, offset: u64) -> IResult<Self>
    where
        T: Read + Seek,
    {
        reader.seek(SeekFrom::Start(offset))?;

        let mut mo = MOBIDOCHeader::default();
        mo.compression = reader.read_u16()?;
        reader.read_u16()?;
        mo.length = reader.read_u32()?;
        mo.record_count = reader.read_u16()?;
        mo.record_size = reader.read_u16()?;
        mo.position = reader.read_u32()?;
        if mo.compression == 17480 {
            mo.encrypt_type = ((mo.position >> 8) & 0xffff) as u16;
        }
        Ok(mo)
    }
}

#[derive(Default, Debug)]
pub(crate) struct MOBIHeader {
    // the characters M O B I
    ///  the length of the MOBI header, including the previous 4 bytes
    header_len: u32,
    /// The kind of Mobipocket file this is
    /// 2 Mobipocket Book
    /// 3 PalmDoc Book
    /// 4 Audio
    /// 232 mobipocket? generated by kindlegen1.2
    /// 248 KF8: generated by kindlegen2
    /// 257 News
    /// 258 News_Feed
    /// 259 News_Magazine
    /// 513 PICS
    /// 514 WORD
    /// 515 XLS
    /// 516 PPT
    /// 517 TEXT
    /// 518 HTML
    mobi_type: u32,
    /// 1252 = CP1252 (WinLatin1); 65001 = UTF-8
    text_encoding: u32,
    /// Some kind of unique ID number (random?)
    unique_id: u32,
    /// Version of the Mobipocket format used in this file.
    file_version: u32,
    /// Section number of orthographic meta index. 0xFFFFFFFF if index is not available.
    ortographic_index: u32,
    /// Section number of inflection meta index. 0xFFFFFFFF if index is not available.
    inflection_index: u32,
    /// 0xFFFFFFFF if index is not available.
    index_names: u32,
    /// 0xFFFFFFFF if index is not available.
    index_keys: u32,
    /// Section number of extra N meta index. 0xFFFFFFFF if index is not available.
    extra_index: [u32; 6],
    /// First record number (starting with 0) that's not the book's text
    first_non_book_index: u32,
    /// Offset in record 0 (not from start of file) of the full name of the book
    full_name_offset: u32,

    ///  Length in bytes of the full name of the book
    full_name_length: u32,
    ///  Book locale code. Low byte is main language 09= English, next byte is dialect, 08 = British, 04 = US. Thus US English is 1033, UK English is 2057.
    locale: u32,
    /// Input language for a dictionary
    input_language: u32,
    /// Output language for a dictionary
    output_language: u32,
    /// Minimum mobipocket version support needed to read this file.
    min_version: u32,
    /// First record number (starting with 0) that contains an image. Image records should be sequential.
    first_image_index: u32,
    /// The record number of the first huffman compression record.
    huffman_record_offset: u32,
    /// The number of huffman compression records.
    huffman_record_count: u32,
    ///     
    huffman_table_offset: u32,
    ///     
    huffman_table_length: u32,
    /// bitfield. if bit 6 (0x40) is set, then there's an EXTH record
    /// 当从低到高第六位为1，代表有EXTH，与其他bit无关
    exth_flags: u32,
    /// 32 unknown bytes, if MOBI is long enough
    unknown_0: [u8; 8],
    /// Use 0xFFFFFFFF
    unknown_1: u32,
    /// Offset to DRM key info in DRMed files. 0xFFFFFFFF if no DRM
    /// 实际 没有drm这里是0？待测试
    drm_offset: u32,
    /// Number of entries in DRM info. 0xFFFFFFFF if no DRM
    drm_count: u32,
    /// Number of bytes in DRM info.
    drm_size: u32,
    /// Some flags concerning the DRM info.
    drm_flags: u32,
    /// Bytes to the end of the MOBI header, including the following if the header length >= 228 (244 from start of record).Use 0x0000000000000000.
    unknown_2: u64,

    /// Number of first text record. Normally 1.
    first_content_record_number: u16,
    /// Number of last image record or number of last text record if it contains no images. Includes Image, DATP, HUFF, DRM.
    last_content_record_number: u16,
    /// FCIS record count? Use 0x00000001.
    unknown_3: u32,
    ///
    fcis_record_number: u32,
    /// Use 0x00000001.
    unknown_4: u32,
    ///
    flis_record_number: u32,
    /// Use 0x00000001.flis record count?
    unknown_5: u32,
    /// Use 0x0000000000000000.
    unknown_6: u64,
    /// Use 0xFFFFFFFF.
    unknown_7: u32,
    /// Use 0x00000000.
    first_compilation_data_section_count: u32,
    /// Use 0xFFFFFFFF.
    number_of_compilation_data_sections: u32,
    /// Use 0xFFFFFFFF.
    unknown_8: u32,
    /// A set of binary flags, some of which indicate extra data at the end of each text block. This only seems to be valid for Mobipocket format version 5 and 6 (and higher?), when the header length is 228 (0xE4) or 232 (0xE8).
    /// bit 1 (0x1) : <extra multibyte bytes><size>
    /// bit 2 (0x2) : <TBS indexing description of this HTML record><size>
    /// bit 3 (0x4) : <uncrossable breaks><size>
    /// Setting bit 2 (0x2) disables <guide><reference type="start"> functionality.
    extra_record_data_flags: u32,
    /// (If not 0xFFFFFFFF)The record number of the first INDX record created from an ncx file.
    indx_record_offset: u32,
    /// 0xFFFFFFFF In new MOBI file, the MOBI header length is 256, skip this to EXTH header.
    unknown_9: u32,
    /// 0xFFFFFFFF In new MOBI file, the MOBI header length is 256, skip this to EXTH header.
    unknown_10: u32,
    /// 0xFFFFFFFF In new MOBI file, the MOBI header length is 256, skip this to EXTH header.
    unknown_11: u32,
    /// 0xFFFFFFFF In new MOBI file, the MOBI header length is 256, skip this to EXTH header.
    unknown_12: u32,
    /// 0xFFFFFFFF In new MOBI file, the MOBI header length is 256, skip this to EXTH header.
    unknown_13: u32,
    /// 0 In new MOBI file, the MOBI header length is 256, skip this to EXTH header, MOBI Header length 256, and add 12 bytes from PalmDOC Header so this index is 268.
    unknown_14: u32,
}

impl MOBIHeader {
    pub fn load<T>(reader: &mut T) -> IResult<Self>
    where
        T: Read + Seek,
    {
        let mut header = Self::default();

        let start = reader.stream_position()?;

        if "MOBI" != reader.read_string(4)? {
            return Err(IError::UnsupportedArchive("not a mobi file"));
        }

        header.header_len = reader.read_u32()?;
        header.mobi_type = reader.read_u32()?;
        header.text_encoding = reader.read_u32()?;
        header.unique_id = reader.read_u32()?;
        header.file_version = reader.read_u32()?;
        header.ortographic_index = reader.read_u32()?;
        header.inflection_index = reader.read_u32()?;
        header.index_names = reader.read_u32()?;
        header.index_keys = reader.read_u32()?;
        reader.read_exact_u32(&mut header.extra_index)?;
        header.first_non_book_index = reader.read_u32()?;
        // 规范里要求 offset 是从 record 0 开始，也就是当前这个 mobi header，但是为了方便，这里给改成从 文件开头开始索引
        header.full_name_offset = reader.read_u32()?;
        header.full_name_offset += start as u32;
        // palm_doc 的16个字节也在 record 0 里面，所以还需要去掉
        header.full_name_offset -= 16;

        header.full_name_length = reader.read_u32()?;
        header.locale = reader.read_u32()?;
        header.input_language = reader.read_u32()?;
        header.output_language = reader.read_u32()?;
        header.min_version = reader.read_u32()?;
        header.first_image_index = reader.read_u32()?;
        header.huffman_record_offset = reader.read_u32()?;
        header.huffman_record_count = reader.read_u32()?;
        header.huffman_table_offset = reader.read_u32()?;
        header.huffman_table_length = reader.read_u32()?;
        header.exth_flags = reader.read_u32()?;

        reader.seek(SeekFrom::Current(32))?;
        // reader.read_exact(&mut header.unknown_0)?;

        header.unknown_1 = reader.read_u32()?;
        header.drm_offset = reader.read_u32()?;
        header.drm_count = reader.read_u32()?;
        header.drm_size = reader.read_u32()?;
        header.drm_flags = reader.read_u32()?;
        header.unknown_2 = reader.read_u64()?;
        header.first_content_record_number = reader.read_u16()?;
        header.last_content_record_number = reader.read_u16()?;
        header.unknown_3 = reader.read_u32()?;
        header.fcis_record_number = reader.read_u32()?;
        header.unknown_4 = reader.read_u32()?;
        header.flis_record_number = reader.read_u32()?;
        header.unknown_5 = reader.read_u32()?;
        header.unknown_6 = reader.read_u64()?;
        header.unknown_7 = reader.read_u32()?;
        header.first_compilation_data_section_count = reader.read_u32()?;
        header.number_of_compilation_data_sections = reader.read_u32()?;
        header.unknown_8 = reader.read_u32()?;
        header.extra_record_data_flags = reader.read_u32()?;
        header.indx_record_offset = reader.read_u32()?;

        // 有的 mobi header长度是256，有的232，所以有可能需要跳过一些字节
        reader.seek(SeekFrom::Start(start + header.header_len as u64))?;
        Ok(header)
    }
}

#[derive(Default, Debug)]
pub(crate) struct EXTHRecord {
    /// Exth Record type. Just a number identifying what's stored in the record
    _type: u32,
    /// length of EXTH record = L , including the 8 bytes in the type and length fields
    len: u32,
    /// Data，L - 8
    data: Vec<u8>,
}
impl EXTHRecord {
    fn load<T: ReadCount>(reader: &mut T) -> IResult<Self> {
        let mut v = Self::default();
        v._type = reader.read_u32()?;
        v.len = reader.read_u32()?;

        reader.take((v.len - 8) as u64).read_to_end(&mut v.data)?;

        Ok(v)
    }
}

///
///
/// 参见 [https://wiki.mobileread.com/wiki/MOBI#EXTH_Header]
///
#[derive(Default, Debug)]
pub(crate) struct EXTHHeader {
    // the characters E X T H
    // identifier: [u8; 4],
    /// the length of the EXTH header, including the previous 4 bytes - but not including the final padding.
    len: u32,
    /// The number of records in the EXTH header. the rest of the EXTH header consists of repeated EXTH records to the end of the EXTH length.
    record_count: u32,
    /// 不定长度的 record,
    record_list: Vec<EXTHRecord>, // 多余的字节均为无用填充，跳过即可
}

macro_rules! simple_utf8 {
    ($expr:expr) => {{
        match String::from_utf8($expr.clone()) {
            Ok(va) => $crate::common::unescape_html(va.as_str()),
            Err(e) => {
                return Err($crate::common::IError::Utf8(e));
            }
        }
    }};
}

impl EXTHHeader {
    ///
    /// [reader] reader
    /// [exth_flas] 如果 &0x40 != 0x40，返回None
    pub fn load<T>(reader: &mut T, exth_flags: u32) -> IResult<Option<Self>>
    where
        T: ReadCount,
    {
        if exth_flags & 0x40 != 0x40 {
            return Ok(None);
        }

        let mut v = Self::default();

        if "EXTH" != reader.read_string(4)? {
            return Err(IError::InvalidArchive("not a exth"));
        }

        v.len = reader.read_u32()?;
        v.record_count = reader.read_u32()?;
        for _ in 0..v.record_count {
            v.record_list.push(EXTHRecord::load(reader)?);
        }

        // 跳过一定padding字节数,规则是保证 header整体长度一定是4的整数，如果实际数据不够，则填充到4的整数
        // Null bytes to pad the EXTH header to a multiple of four bytes (none if the header is already a multiple of four). This padding is not included in the EXTH header length.
        let skip = 4 - v.len % 4;
        if skip != 4 {
            reader.seek(SeekFrom::Current(skip as i64))?;
        }
        Ok(Some(v))
    }

    fn get_cover_offset(&self) -> Option<u64> {
        self.record_list
            .iter()
            .find(|x| x._type == 201)
            .map(|f| vec_u8_to_u64(&f.data))
            .filter(|f| f < &0xffffffff)
    }

    fn get_thumbnail_offset(&self) -> Option<u64> {
        self.record_list
            .iter()
            .find(|x: &&EXTHRecord| x._type == 202)
            .map(|f| vec_u8_to_u64(&f.data))
            .filter(|f| f < &0xffffffff)
    }

    /// 解析元数据
    fn get_meta(&self) -> IResult<BookInfo> {
        let mut info = BookInfo::default();
        for ele in &self.record_list {
            match ele._type {
                100 => {
                    let v = simple_utf8!(ele.data);
                    // 暂时只考虑utf-8编码
                    info.append_creator(v.as_str());
                }
                101 => {
                    info.publisher = Some(simple_utf8!(ele.data));
                }
                103 => {
                    info.description = Some(simple_utf8!(ele.data));
                }
                104 => {
                    info.identifier = simple_utf8!(ele.data);
                }
                105 => {
                    info.subject = Some(simple_utf8!(ele.data));
                }
                106 => {
                    info.date = Some(simple_utf8!(ele.data));
                }
                108 => {
                    info.contributor = Some(simple_utf8!(ele.data));
                }
                503 => {
                    info.title = simple_utf8!(ele.data);
                }
                _ => {}
            }
        }
        Ok(info)
    }
}

#[derive(Debug, Default)]
struct INDXRecord {
    /// 在之前还有 4个字节的 Identifier，固定为I N D X
    /// the length of the INDX header, including the previous 4 bytes
    len: u32,
    _type: u32,
    /// 前面还有8个无用字节
    /// the offset to the IDXT section
    idxt_start: u32,
    /// the number of index records
    index_count: u32,
    /// 1252 = CP1252 (WinLatin1); 65001 = UTF-8
    index_encoding: u32,
    /// the language code of the index
    index_language: u32,
    /// the number of index entries
    total_index_count: u32,
    /// the offset to the ORDT section
    ordt_start: u32,
    /// the offset to the LIGT section
    ligt_start: u32,
    /// 文档没有描述
    ligt_count: u32,
    /// 文档没有描述
    cncx_count: u32,
}

impl INDXRecord {
    pub fn load<T>(reader: &mut T) -> IResult<Self>
    where
        T: ReadCount,
    {
        let mut v = Self::default();
        let start = reader.stream_position()?;

        if reader.read_string(4)? != "INDX" {
            return Err(IError::InvalidArchive("not a indx"));
        }

        v.len = reader.read_u32()?;
        v._type = reader.read_u32()?;
        reader.skip(8)?;
        v.idxt_start = reader.read_u32()?;
        v.index_count = reader.read_u32()?;
        v.index_encoding = reader.read_u32()?;
        v.index_language = reader.read_u32()?;
        v.total_index_count = reader.read_u32()?;
        v.ordt_start = reader.read_u32()?;
        v.ligt_start = reader.read_u32()?;
        v.ligt_count = reader.read_u32()?;
        v.cncx_count = reader.read_u32()?;

        // 整个indx不止文档里的56个字节，多出的长度应该就是 index value
        reader.skip((v.len - 56).into())?;
        // reader.seek(SeekFrom::Start(start + v.len as u64))?;

        Ok(v)
    }
}

/// 格式化时间戳
pub(crate) fn do_time_format(value: u32) -> String {
    if value & 0x80000000 == 0x80000000 {
        crate::common::do_time_display((value & 0x7fffffff) as u64, 1904)
    } else {
        crate::common::time_display(value as u64)
    }
}

/// 计算一个数字 有多少位是1
fn count_bit(v: u32) -> usize {
    let mut count = 0;
    let mut nv = v;
    while (nv) > 0 {
        if (nv & 1) == 1 {
            count += 1;
        }
        nv = nv >> 1;
    }
    return count;
}
/// 统计有多少位0 ，因为不同类型循环次数不同，会非常影响结果
fn count_unset_end(v: u8) -> usize {
    let mut count = 0;
    let mut x = v;
    while (x & 1) == 0 {
        x = x >> 1;
        count += 1;
    }
    count
}
fn get_var_len(byte: &[u8]) -> (usize, usize) {
    let mut value: usize = 0;
    let mut length: usize = 0;
    for ele in byte {
        value = (value << 7) | ((ele & 0b111_1111) as usize);
        length += 1;
        if ele & 0b1000_0000 >= 1 {
            break;
        }
    }
    (value, length)
}

// https://wiki.mobileread.com/wiki/PDB#Intro_to_the_Database_format
// https://wiki.mobileread.com/wiki/PDB#Palm_Database_Format

// fn u32_to_string(value:[u32])->String{

//     let mut v = [0u8;4];
//     v[0] = (value >> 24 & 0xff) as u8;
//     v[1]=(value >> 16 & 0xff) as u8;
//     v[2] = (value >> 8 & 0xff) as u8;
//     v[3] = (value & 0xff) as u8;

//     String::from_utf8(v.to_vec()).unwrap_or(String::new())

// }

fn u8_to_string<const N: usize>(v: [u8; N]) -> String {
    // let mut v = [0u8;4];
    // v[0] = (value >> 24 & 0xff) as u8;
    // v[1]=(value >> 16 & 0xff) as u8;
    // v[2] = (value >> 8 & 0xff) as u8;
    // v[3] = (value & 0xff) as u8;

    String::from_utf8(v.to_vec()).unwrap_or(String::new())
}

impl std::fmt::Display for PDBHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f,"PDBHeader {{ name: '{}', attribute: {}, version: {}, createion_date: {}, modify_date: {}, last_backup_date: {}, modification_number: {}, app_info_id: {}, sort_info_id: {}, _type: {}, creator: {}, unique_id_seed: {}, next_record_list_id: {}, number_of_records: {}, record_info_list: {:?}, record_list: [] }}"
            ,u8_to_string(self.name)
        ,self.attribute
        ,self.version
        ,do_time_format(self.createion_date)
        ,do_time_format(self.modify_date)
        ,do_time_format(self.last_backup_date)
        ,self.modification_number
        ,self.app_info_id
        ,self.sort_info_id
        ,u8_to_string(self._type)
        ,u8_to_string(self.creator)
        ,self.unique_id_seed
        ,self.next_record_list_id
        ,self.number_of_records
        ,self.record_info_list

            )
    }
}
#[derive(Debug)]
struct NCX {
    index: usize,
    offset: Option<usize>,
    size: Option<usize>,
    label: String,
    heading_lebel: usize,
    pos: usize,
    parent: Option<usize>,
    first_child: Option<usize>,
    last_child: Option<usize>,
}

mod Ext {
    use std::{
        collections::HashMap,
        io::{Read, Seek, SeekFrom},
    };

    use crate::common::{IError, IResult};

    pub(crate) trait NCXExt {
        fn get_value(&self, index: u8) -> Option<usize>;
        fn get_value_or(&self, index: u8, default: usize) -> usize;
    }

    impl NCXExt for HashMap<u8, Vec<usize>> {
        fn get_value(&self, index: u8) -> Option<usize> {
            self.get(&index).and_then(|f| f.get(0)).map(|f| f.clone())
        }
        fn get_value_or(&self, index: u8, default: usize) -> usize {
            self.get_value(index).unwrap_or(default)
        }
    }
    pub(crate) trait ReadCount: Read + Seek {
        fn read_u8(&mut self) -> IResult<u8> {
            let mut out = [0u8; 1];
            self.read_exact(&mut out)?;
            Ok(out[0])
        }
        fn read_u16(&mut self) -> IResult<u16> {
            let mut out = [0u8; 2];
            let mut res: u16 = 0;
            self.read_exact(&mut out)?;
            for i in 0..out.len() {
                res = res << 8;
                res = res | out[i] as u16;
            }
            Ok(res)
        }
        fn read_u32(&mut self) -> IResult<u32> {
            let mut out = [0u8; 4];
            let mut res: u32 = 0;
            self.read_exact(&mut out)?;
            for i in 0..out.len() {
                res = res << 8;
                res = res | out[i] as u32;
            }

            Ok(res)
        }
        fn read_u64(&mut self) -> IResult<u64> {
            let mut out = [0u8; 8];
            let mut res: u64 = 0;
            self.read_exact(&mut out)?;

            for i in 0..out.len() {
                res = res << 8;
                res = res | out[i] as u64;
            }

            Ok(res)
        }

        fn read_exact_u32<const N: usize>(&mut self, value: &mut [u32; N]) -> IResult<()> {
            for i in 0..value.len() {
                value[i] = self.read_u32()?;
            }
            Ok(())
        }

        fn read_string(&mut self, limit: u64) -> IResult<String> {
            let mut out = String::new();
            self.take(limit).read_to_string(&mut out)?;

            Ok(out)
        }

        fn skip(&mut self, limit: u64) -> IResult<u64> {
            self.seek(SeekFrom::Current(limit as i64))
                .map_err(|f| IError::Io(f))
        }
    }

    impl<R: Read + Seek> ReadCount for R {}
}

use Ext::ReadCount;

impl NCX {
    fn from(index: usize, label: String, map: &HashMap<u8, Vec<usize>>) -> Self {
        use Ext::NCXExt;

        NCX {
            index: index,
            offset: map.get_value(1),
            size: map.get_value(2),
            label,
            heading_lebel: map.get_value_or(4, 0),
            pos: map.get_value_or(6, 0),
            parent: map.get_value(21),
            first_child: map.get_value(22),
            last_child: map.get_value(23),
        }
    }
}
/// 判断是否是mobi
pub fn is_mobi<T>( value:&mut T) -> IResult<bool>
where
    T: Read + Seek,
{
    value.seek(SeekFrom::Start(60))?;
    let mut buf = Vec::new();
    let _ = value.take(8).read_to_end(&mut buf)?;

    if buf != b"BOOKMOBI" {
        return Ok(false);
    }
    Ok(true)
}

pub struct MobiReader<T> {
    reader: BufReader<T>,
    pub(crate) pdb_header: PDBHeader,
    pub(crate) mobi_doc_header: MOBIDOCHeader,
    pub(crate) mobi_header: MOBIHeader,
    pub(crate) exth_header: Option<EXTHHeader>,
    /// 原始文本缓存
    text_cache: Option<Vec<u8>>,
}

impl<T: Read + Seek> MobiReader<T> {
    pub fn new(v: T) -> IResult<MobiReader<T>> {
        // let fs = std::fs::File::open(file)?;
        let mut reader = BufReader::new(v);

        // 校验基础格式
        reader.seek(SeekFrom::Start(60))?;

        if reader.read_string(8)? != "BOOKMOBI" {
            return Err(IError::InvalidArchive("not a mobi"));
        }
        reader.seek(SeekFrom::Start(0))?;

        let pdb_header = PDBHeader::load(&mut reader)?;
        let mobi_doc_header =
            MOBIDOCHeader::load(&mut reader, pdb_header.record_info_list[0].offset as u64)?;
        let mobi_header = MOBIHeader::load(&mut reader)?;

        let exth_header = EXTHHeader::load(&mut reader, mobi_header.exth_flags)?;

        Ok(MobiReader {
            reader: reader,
            pdb_header: pdb_header,
            mobi_doc_header: mobi_doc_header,
            mobi_header: mobi_header,
            exth_header,
            text_cache: None,
        })
    }

    /// 解析书籍元数据
    pub(crate) fn read_meta_data(&mut self) -> IResult<BookInfo> {
        let current = self.reader.stream_position()?;

        self.reader
            .seek(SeekFrom::Start(self.mobi_header.full_name_offset as u64))?;
        let mut title = String::new();

        self.reader
            .get_mut()
            .take(self.mobi_header.full_name_length as u64)
            .read_to_string(&mut title)?;
        self.reader.seek(SeekFrom::Start(current))?;
        // self.reader.read_to_string(buf)

        if let Some(exth) = &self.exth_header {
            return exth.get_meta();
        }
        let mut info = BookInfo::default();
        info.title = title;
        Ok(info)
    }
    /// record的第一个字节的offset
    ///
    /// (当前的offset，下一个的offset)
    ///
    pub(crate) fn seek_record_offset(&mut self, index: u32) -> IResult<(u64, u64)> {
        let offset = self.pdb_header.record_info_list[index as usize].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;

        Ok((
            offset,
            self.pdb_header.record_info_list[(index + 1) as usize].offset as u64,
        ))
    }

    pub(crate) fn get_var_len(byte: &[u8]) -> (usize, usize) {
        let mut value: usize = 0;
        let mut length: usize = 0;
        for ele in byte {
            value = (value << 7) | ((ele & 0b111_1111) as usize);
            length += 1;
            if ele & 0b1000_0000 >= 1 {
                break;
            }
        }
        (value, length)
    }

    /// 从文本中获取目录信息
    /// [sec] 分节信息，filepos
    pub(crate) fn read_nav_from_text(
        &mut self,
        sec: &[TextSection],
    ) -> IResult<Option<Vec<MobiNav>>> {
        let raw = self.read_text_raw()?;
        let file_pos = read_guide_filepos(&raw[..])?;

        if let Some(toc) = file_pos.map_or(None, |v| sec.iter().find(|s| s.end > v)) {
            return read_nav_xml(toc.data.as_bytes().to_vec()).map(|s| Some(s));
        }

        Ok(None)
    }

    /// 解析目录
    pub(crate) fn read_nav(&mut self) -> IResult<()> {
        if self.mobi_header.indx_record_offset < 0xffffffff {
            self.seek_record_offset(self.mobi_header.indx_record_offset)?;

            let indx = INDXRecord::load(&mut self.reader)?;
            println!("postion = {}", self.reader.stream_position()?);

            println!("{:?}", indx);

            if self.reader.read_string(4)? != "TAGX" {
                return Err(IError::InvalidArchive("not a tagx"));
            }
            let mut tagx_table: Vec<[u8; 4]> = Vec::new();
            let len = self.reader.read_u32()?;
            // the number of control bytes
            let tagx_control_byte_count = self.reader.read_u32()?;

            for _ in 0..((len - 12) / 4) {
                // 四个字节的含义
                // The tag table entries are multiple of 4 bytes. The first byte is the tag, the second byte the number of values, the third byte the bit mask and the fourth byte indicates the end of the control byte. If the fourth byte is 0x01, all other bytes of the entry are zero.
                let mut v = [0u8; 4];
                self.reader.read_exact(&mut v)?;

                tagx_table.push(v);
            }

            // 剩余字段的解析方式文档里没有再多描述，只能翻译别的项目代码
            let mut cntx = HashMap::new();
            let mut cncx_record_offset = 0;
            for i in 0..indx.cncx_count {
                println!(
                    "offset = {}",
                    self.mobi_header.indx_record_offset + indx.index_count + 1 + i
                );
                let (now, offset) = self.seek_record_offset(
                    self.mobi_header.indx_record_offset + indx.index_count + 1 + i,
                )?;

                let mut record = Vec::new();
                self.reader
                    .get_mut()
                    .take(offset - now)
                    .read_to_end(&mut record)?;

                let mut pos = 0;

                println!(
                    "before cntx {} now ={},offset = {}",
                    self.reader.stream_position()?,
                    now,
                    offset
                );
                while pos < record.len() {
                    let index = pos;
                    let bytes = &record[pos..(pos + 4)];
                    // println!("bytes = {}",bytes[0]);
                    // self.reader.get_mut().take(1).read_to_end(&mut bytes)?;
                    println!("bytes = {}", bytes[0]);
                    println!(
                        "cntx {} now ={},offset = {}",
                        self.reader.stream_position()?,
                        now,
                        offset
                    );

                    let (value, length) = Self::get_var_len(&bytes[0..]);
                    println!("cncx {} {}", value, length);
                    pos += length;
                    let result = &record[pos..(pos + value)];
                    pos += value;
                    cntx.insert(
                        cncx_record_offset + index,
                        String::from_utf8(result.to_vec()).unwrap_or(String::new()),
                    );
                }
                cncx_record_offset += 0x10000;
            }

            println!("{:?} {}", cntx, cntx.len());
            let mut table = Vec::new();
            for i in 0..indx.index_count {
                let (start, _) =
                    self.seek_record_offset(self.mobi_header.indx_record_offset + 1 + i)?;
                let n_index = INDXRecord::load(&mut self.reader)?;

                for j in 0..n_index.index_count {
                    let offset_offset = (n_index.idxt_start + 4 + 2 * j) as u64;
                    self.reader.seek(SeekFrom::Start(start + offset_offset))?;
                    let offset = self.reader.read_u16()? as u64;
                    self.reader.seek(SeekFrom::Start(start + offset as u64))?;

                    let length = self.reader.read_u8()?;
                    self.reader
                        .seek(SeekFrom::Start(start + offset as u64 + 1))?;
                    println!("name offset = {}", offset);
                    let _name = self.reader.read_string(length as u64)?;

                    let mut tags = Vec::new();

                    let start_pos = offset + 1 + length as u64;
                    let mut control_byte_index = 0;
                    let mut pos = start_pos + tagx_control_byte_count as u64;
                    #[inline]
                    fn get_array_var_len(
                        reader: &mut impl ReadCount,
                        start: u64,
                        pos: u64,
                    ) -> IResult<(usize, usize)> {
                        reader.seek(SeekFrom::Start(start + pos))?;
                        let mut buf = [0u8; 4];
                        reader.read_exact(&mut buf)?;
                        Ok(get_var_len(&buf))
                    }

                    for ele in &tagx_table {
                        let tag = ele[0];
                        let num_values = ele[1];
                        let mask = ele[2];
                        let end = ele[3];

                        if end & 1 >= 1 {
                            control_byte_index += 1;
                            continue;
                        }

                        let offset: u64 = start_pos + control_byte_index;
                        self.reader.seek(SeekFrom::Start(start + offset))?;
                        let value = self.reader.read_u8()? & mask;
                        if value == mask {
                            if count_bit(mask as u32) > 1 {
                                let (value, length) =
                                    get_array_var_len(&mut self.reader, start, pos)?;

                                tags.push((tag, None, Some(value), num_values));
                                pos += length as u64;
                            } else {
                                tags.push((tag, Some(1), None, num_values));
                            }
                        } else {
                            tags.push((
                                tag,
                                Some(value >> count_unset_end(mask)),
                                None,
                                num_values,
                            ));
                        }
                    }

                    println!("tags = {:?}", tags);

                    let mut tag_map: HashMap<u8, Vec<usize>> = HashMap::new();
                    for (tag, value_count, value_bytes, num_values) in tags {
                        let mut values = Vec::new();
                        if let Some(v) = value_count {
                            for _m in 0..(v as u32 * (num_values as u32)) {
                                let (value, length) =
                                    get_array_var_len(&mut self.reader, start, pos)?;

                                values.push(value);
                                pos += length as u64;
                            }
                        } else {
                            let mut count: usize = 0;
                            while count < value_bytes.unwrap() {
                                let (value, length) =
                                    get_array_var_len(&mut self.reader, start, pos)?;

                                values.push(value);
                                pos += length as u64;
                                count += length;
                            }
                        }
                        tag_map.insert(tag, values);
                    }
                    table.push((
                        cntx.get(tag_map.get(&3).and_then(|f| f.get(0)).unwrap_or(&0)),
                        tag_map,
                    ));
                }
            }
            println!("{:?}", table);

            let items = table.iter().enumerate().map(|(index, (name, map))| {
                return NCX::from(
                    index,
                    if let Some(n) = name {
                        n.to_string()
                    } else {
                        String::new()
                    },
                    &map,
                );
            });

            for ele in items {
                println!("NCX {:?}", ele);
            }

            // println!("NCX {:?}",items);
        }
        Ok(())
    }

    /// 解析封面
    pub(crate) fn read_cover(&mut self) -> IResult<Option<Cover>> {
        if let Some(exth) = &self.exth_header {
            if let Some(offset) = exth.get_cover_offset().or(exth.get_thumbnail_offset()) {
                let (now, next) = self
                    .seek_record_offset(self.mobi_header.first_image_index + offset as u32)
                    .unwrap();

                let mut image = Vec::new();
                self.reader
                    .get_mut()
                    .take(next - now)
                    .read_to_end(&mut image)
                    .unwrap();
                return Ok(Some(Cover(image)));
            }
        }
        Ok(None)
    }

    /// 获取所有图片
    pub(crate) fn read_all_image(&mut self) -> IResult<Vec<MobiAssets>> {
        let text = self.read_text_raw()?;

        let index = read_image_recindex_from_html(text.as_slice())?;

        Ok(index
            .iter()
            .map(|f| {
                let (now, next) = self
                    .seek_record_offset(self.mobi_header.first_image_index + *f as u32)
                    .unwrap();

                let mut image = Vec::new();
                self.reader
                    .get_mut()
                    .take(next - now)
                    .read_to_end(&mut image)
                    .unwrap();

                return MobiAssets {
                    _file_name: format!("{}.{}",f,get_suffix(image.as_slice())),
                    media_type: String::new(),
                    _data: Some(image),
                    recindex: f.clone(),
                };
            })
            .collect())
    }

    /// 读取文本，注意这里并不将文本解码，依然保留原始字节
    pub(crate) fn read_text_raw(&mut self) -> IResult<Vec<u8>> {
        if let Some(v) = &self.text_cache {
            return Ok(v.clone());
        }
        // 获取所有text record
        let mut text: Vec<u8> = Vec::new();
        // let reader = &mut self.reader;
        let tail_circle_count = count_bit(self.mobi_header.extra_record_data_flags >> 1);

        // 第0个是header，所以从1开始
        for i in 1..(self.mobi_doc_header.record_count + 1) {
            let mut record: Vec<u8> = Vec::new();

            let (start, end) = self.seek_record_offset(i as u32)?;
            let len = end - start;
            // self.reader.seek(SeekFrom::Start(start))?;
            self.reader.get_mut().take(len).read_to_end(&mut record)?;

            // 处理尾巴
            let size = get_mobi_variable_width_len(
                &record,
                tail_circle_count,
                self.mobi_header.extra_record_data_flags,
            );

            for _ in 0..size {
                record.remove(record.len() - 1);
            }

            if self.mobi_doc_header.compression == 2 {
                // 解压缩
                record = uncompression_lz77(&record);
            }

            text.append(&mut record);
        }

        self.text_cache = Some(text.clone());

        return Ok(text);
    }

    /// 解码文本
    fn decode_text(&self, data: &[u8]) -> IResult<String> {
        if self.mobi_header.text_encoding == 1252 {
            // iSO-8859-1
            Ok(data.iter().map(|&c| c as char).collect())
        } else {
            String::from_utf8(data.to_vec()).map_err(|e| IError::Utf8(e))
        }
    }

    /// 加载文本，将文本分节，读取图片等信息
    pub(crate) fn load_text(&mut self) -> IResult<Vec<TextSection>> {
        let text = self.read_text_raw()?;

        // 查找子串
        let sub_bytes = b"<mbp:pagebreak/>";

        let mut i = 0;
        let mut prev = TextSection {
            index: 0,
            start: 0,
            end: 0,
            data: String::new(),
        };
        let mut pos = vec![];
        while i < text.len() {
            let mut now = &text[i];

            let mut j = 0;
            while j < sub_bytes.len() {
                if &sub_bytes[j] != now {
                    // i += j;
                    break;
                } else {
                    i += 1;
                    now = &text[i];
                }
                j += 1;
            }

            if j == sub_bytes.len() {
                // let prev = pos.last_mut().unwrap();

                prev.end = i - sub_bytes.len();

                prev.data = self.decode_text(&text[(prev.start + sub_bytes.len())..prev.end])?;
                pos.push(prev);
                prev = TextSection {
                    index: pos.last().unwrap().index + 1,
                    start: pos.last().unwrap().end,
                    end: 0,
                    data: String::new(),
                }
                // pos.push(i - sub_bytes.len());
            }

            i += 1;
        }
        prev.end = i;
        prev.data = self.decode_text(&text[(prev.start + sub_bytes.len())..i])?;
        pos.push(prev);

        Ok(pos)
    }
}

/// 文本分节
#[derive(Debug, Default)]
pub(crate) struct TextSection {
    pub(crate) index: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// 被解码后可阅读的文本数据
    pub(crate) data: String,
}

///
/// [https://wiki.mobileread.com/wiki/MOBI#Variable-width_integers]
/// 看了好几遍，都还是没看懂文档是什么意思，只能把别的项目里的代码给翻译过来
///
fn get_mobi_variable_width_len(data: &[u8], tail_circle_count: usize, flag: u32) -> usize {
    let mut n_data = &data[..];
    for _ in 0..tail_circle_count {
        let res = buffer_get_var_len(n_data) as usize;
        n_data = &n_data[..(n_data.len() - res)];
    }

    if flag & 1 > 0 {
        let a = (n_data[n_data.len() - 1] & 0b11) + 1;
        n_data = &n_data[..(n_data.len() - a as usize)];
    }

    data.len() - n_data.len()
}

fn buffer_get_var_len(data: &[u8]) -> u32 {
    let array = &data[data.len() - 4..data.len()];
    let mut value: u32 = 0;
    for ele in array {
        if ele & 0b1000_0000 > 0 {
            value = 0;
        }
        let v: u32 = (*ele).into();
        value = (value << 7) | (v & 0b111_1111)
    }

    return value;
}

/// 解压缩
fn uncompression_lz77(data: &[u8]) -> Vec<u8> {
    let length = data.len();
    let mut offset = 0;
    let mut buffer = Vec::new();

    while offset < length {
        let char = data[offset];
        offset += 1;

        if char == 0 {
            buffer.push(char);
        } else if char <= 8 {
            for i in offset..(offset + char as usize) {
                buffer.push(data[i]);
            }
            offset += char as usize;
        } else if char <= 0x7f {
            buffer.push(char);
        } else if char <= 0xbf {
            let next = data[offset];
            offset += 1;
            let cc = char as usize;
            let distance = ((((cc << 8) | next as usize) >> 3) & 0x7ff) as usize;
            let lz_length = (next & 0x7) + 3;
            let mut buffer_size = buffer.len();

            for _ in 0..lz_length {
                buffer.push(buffer[buffer_size - distance]);
                buffer_size += 1;
            }
        } else {
            buffer.push(32);
            buffer.push(char ^ 0x80);
        }
    }

    buffer
}

#[cfg(test)]
mod tests {
    use std::io::Seek;

    use quick_xml::reader;

    use crate::mobi::reader::do_time_format;
    use crate::mobi::reader::is_mobi;

    use super::u8_to_string;

    use super::MobiReader;
    use super::PDBHeader;

    #[test]
    fn d() {
        let m = vec![1, 2, 3, 4, 5, 6, 7, 8];
        println!("{:?}", &m[m.len() - 4..m.len()]);
    }

    #[test]
    fn test_is_mobi() {
        let mut data: Vec<u8> = Vec::new();

        assert_eq!(false, is_mobi(&mut std::io::Cursor::new(&mut data)).unwrap());

        let empty = [0u8; 60];
        data.append(&mut empty.to_vec());
        data.append(&mut b"BOOKMOB".to_vec());
        assert_eq!(false, is_mobi(&mut std::io::Cursor::new(&mut data)).unwrap());

        data.append(&mut b"I".to_vec());
        assert_eq!(true, is_mobi(&mut std::io::Cursor::new(&mut data)).unwrap());

        for _ in 0..60 {
            data.push(0);
        }
        assert_eq!(true, is_mobi(&mut std::io::Cursor::new(&mut data)).unwrap());

        assert_eq!(false, is_mobi(&mut std::io::Cursor::new([0u8; 128])).unwrap());
    }

    #[test]
    fn test_header() {
        // let path = std::env::current_dir().unwrap().join("../dan/dd2.mobi");

        let path = std::env::current_dir().unwrap().join("../dan.mobi");
        println!("dir {:?}", path);
        let mut fs = std::fs::File::open(path.to_str().unwrap()).unwrap();
        let mut h = MobiReader::new(fs).unwrap();

        println!("");

        println!("position = {}", h.reader.stream_position().unwrap());
        let exth = h.exth_header.as_ref().unwrap();
        println!(
            "exth  = {} {} {:?}",
            h.mobi_header.exth_flags,
            h.mobi_header.exth_flags == 64,
            exth
        );

        let book_info = h.read_meta_data().unwrap();

        println!("info = {:?}", book_info);

        println!(
            "{} {}",
            h.pdb_header.createion_date,
            do_time_format(h.pdb_header.createion_date)
        );

        println!("{}", do_time_format(3870581456));
        // println!("{}",
        // h.read_text().unwrap()
        // )
        // ;

        let sec = h.load_text().unwrap();
        println!("sec len = {}", sec.len());

        println!("{}", sec[1].data);
        println!("======");
        println!("{}", sec[46].data);

        let nav = h.read_nav_from_text(&sec[..]).unwrap().unwrap();

        println!("nav = {}", nav.len());

        // h.read_nav().unwrap();

        // h.read_cover()
        // .and_then(|op|op.ok_or(std::io::Error::new(std::io::ErrorKind::InvalidData, "no cover")))
        // .and_then(|mut cover| cover.write("cover1")).unwrap();

        // 尝试读取名字
    }

    // fn read_text(r: &mut MOBIReader) {
    //     let mut reader = &mut r.reader;

    //     for i in 0..r.mobi_doc_header.record_count {
    //         let mut v :Vec<u8> = vec![];
    //         reader.seek(pos)

    //     }
    // }
}
