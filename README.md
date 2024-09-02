# iepub

[EPUB](https://www.w3.org/TR/2023/REC-epub-33-20230525/)格式读写库，
[MOBI](https://wiki.mobileread.com/wiki/MOBI)格式读库，

## EPUB

支持从文件和内存读取和生成EPUB电子书

### 生成

- 可以使用`EpubBook`结构体手动生成epub
- （推荐）使用`EpubBuilder`快速生成

```rust
use iepub::EpubHtml;
use iepub::EpubBuilder;

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

```

### 读取

```rust
use iepub::prelude::{reader::read_from_vec, reader::read_from_file, EpubHtml};
let mut data = Vec::new();// epub的二进制数据

let mut book = read_from_vec(data);
// 从文件读取
let mut bbook = read_from_file("epub格式文件绝对路径");

// 注意，默认情况下读取采用懒加载，上述代码只完成了基础结构读取，包括目录，文件列表等等，具体某个章节或者资源的数据将会推迟到第一次调用`data()`方法时读取
// 例如

let mut chap = book.get_chapter("0.xhtml").unwrap();
let data = chap.data();// 此时将会实际读取并解析文件
let data2 = chap.data();// 第二次不会再读取文件了
```

### 注意事项

`iepub`使用`EpubHtml`来存储章节内容，但是`EpubHtml#data`实际只会存储 html>body 节点内的内容，并且**不包括**body节点的属性(attribute)，其他比如样式表将会存放在其他属性中


## mobi

目前mobi只支持读取

```rust
use iepub::prelude::*;

let path = std::env::current_dir().unwrap().join("1.mobi");
let mut mobi = MobiReader::new(std::fs::File::open(path.to_str().unwrap()).unwrap()).unwrap();
let book = mobi.load().unwrap();
```

## 转换

目前仅支持 mobi -> epub

```rust
use iepub::prelude::*;
let mut book = std::fs::File::open(std::path::PathBuf::from("example.mobi"))
            .map_err(|e| IError::Io(e))
            .and_then(|f| MobiReader::new(f))
            .and_then(|mut f| f.load())
            .unwrap();

let mut epub = mobi_to_epub(&mut book).unwrap();
epub.write("convert.epub").unwrap();
```

## 命令行工具

[tool](https://github.com/inkroom/iepub/releases)目录为命令行工具，支持mobi和epub格式，但是不同格式支持的命令不尽相同

目前支持
- 获取元数据，如标题、作者
- 修改元数据
- 提取封面
- 提取所有图片
- 提取某章节文本
- 获取目录
- 格式转换

可通过`-h`获取使用方法说明
