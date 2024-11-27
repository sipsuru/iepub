//!
//! 实现封面图片的自动生成
//!
use crate::common::IResult;

#[cfg(feature = "cover")]
mod text_width {
    use ab_glyph::PxScale;
    use image::{DynamicImage, GenericImageView, Rgba};

    #[derive(Debug)]
    pub struct Point {
        pub x: u32,
        pub y: u32,
    }

    pub struct ImageCrop {
        pub original: DynamicImage,
    }

    impl ImageCrop {
        pub fn new(text: &str, width: u32, height: u32, font: &impl ab_glyph::Font) -> ImageCrop {
            let mut img = DynamicImage::new_rgb8(width, height);
            // 绘制白色文字
            imageproc::drawing::draw_text_mut(
                &mut img,
                image::Rgba([255u8, 255u8, 255u8, 1u8]),
                0,
                0,
                PxScale {
                    x: width as f32,
                    y: height as f32,
                },
                font,
                text,
            );
            ImageCrop { original: img }
        }

        pub fn calculate_corners(&self) -> (Point, Point) {
            (self.top_left_corner(), self.bottom_right_corner())
        }

        fn is_white(pixel: Rgba<u8>) -> bool {
            pixel[0] == 255 && pixel[1] == 255 && pixel[2] == 255
        }

        fn top_left_corner(&self) -> Point {
            Point {
                x: self.top_left_corner_x(),
                y: self.top_left_corner_y(),
            }
        }

        fn top_left_corner_x(&self) -> u32 {
            for x in 0..(self.original.dimensions().0) {
                for y in 0..(self.original.dimensions().1) {
                    let pixel = self.original.get_pixel(x, y);
                    if Self::is_white(pixel) {
                        return x;
                    }
                }
            }
            panic!("找不到文字，可能是字体中不包含该文字");
        }

        fn top_left_corner_y(&self) -> u32 {
            for y in 0..(self.original.dimensions().1) {
                for x in 0..(self.original.dimensions().0) {
                    let pixel = self.original.get_pixel(x, y);
                    if Self::is_white(pixel) {
                        return y;
                    }
                }
            }
            panic!("找不到文字，可能是字体中不包含该文字");
        }

        fn bottom_right_corner(&self) -> Point {
            Point {
                x: self.bottom_right_corner_x(),
                y: self.bottom_right_corner_y(),
            }
        }

        fn bottom_right_corner_x(&self) -> u32 {
            let mut x = self.original.dimensions().0 as i32 - 1;
            // Using while loop as currently there is no reliable built-in
            // way to use custom negative steps when specifying range
            while x >= 0 {
                let mut y = self.original.dimensions().1 as i32 - 1;
                while y >= 0 {
                    let pixel = self.original.get_pixel(x as u32, y as u32);
                    if Self::is_white(pixel) {
                        return x as u32 + 1;
                    }
                    y -= 1;
                }
                x -= 1;
            }
            panic!("找不到文字，可能是字体中不包含该文字");
        }

        fn bottom_right_corner_y(&self) -> u32 {
            let mut y = self.original.dimensions().1 as i32 - 1;
            // Using while loop as currently there is no reliable built-in
            // way to use custom negative steps when specifying range
            while y >= 0 {
                let mut x = self.original.dimensions().0 as i32 - 1;
                while x >= 0 {
                    let pixel = self.original.get_pixel(x as u32, y as u32);
                    if Self::is_white(pixel) {
                        return y as u32 + 1;
                    }
                    x -= 1;
                }
                y -= 1;
            }
            panic!("找不到文字，可能是字体中不包含该文字");
        }
    }
    #[cfg(test)]
    mod tests {
        use image::Rgba;
        use imageproc::rect::Rect;

        use super::ImageCrop;

        #[test]
        fn test() {
            let f = if std::path::Path::new("target").exists() {
                "target/SourceHanSansSC-Bold.otf"
            } else {
                "../target/SourceHanSansSC-Bold.otf"
            };
            let font = std::fs::read(f).or_else( |_|{
                 tinyget::get("https://github.com/adobe-fonts/source-han-serif/raw/refs/heads/release/SubsetOTF/CN/SourceHanSerifCN-Bold.otf").send().map(|v|{
                    let s =v.as_bytes().to_vec();
                    println!("{} {:?}",s.len(),v.headers);
                    if &s.len().to_string() != v.headers.get("content-length").unwrap_or(&String::new()) && v.status_code !=200 {
                        panic!("字体文件下载失败");
                    }
                    let _ = std::fs::write(f, s.clone());
                    s
                })
            }).unwrap();
            let font = ab_glyph::FontRef::try_from_slice(&font).unwrap();

            let mut img = ImageCrop::new("书", 70, 120, &font);

            let corners = img.calculate_corners();
            let real_width = corners.1.x - corners.0.x;
            println!("real_width {real_width}");
        }
    }
}

/// 生成封面图片
#[cfg(feature = "cover")]
pub(crate) fn gen_cover(book_name: &str, font: &[u8]) -> IResult<Vec<u8>> {
    use ab_glyph::{FontRef, PxScale};
    use image::DynamicImage;
    use text_width::ImageCrop;

    let width = 150;
    let height = 240;
    let margin = 5;

    let text = book_name;

    let mut img = DynamicImage::new_rgb8(width, height);

    let mut row_count: u32 = 0;
    let mut col_count = 0;

    if text.chars().count() % 3 == 0 {
        col_count = 3;
        row_count = (text.chars().count() / 3) as u32;
    } else if text.chars().count() % 2 == 0 {
        col_count = 2;
        row_count = (text.chars().count() / 2) as u32;
    } else {
        // 其他情况，每行两个字
        col_count = 2;
        row_count = ((text.chars().count() / 2) as f32).ceil() as u32;
        row_count = row_count.max(1);
    }

    // 计算一个字可以使用的高度和宽度
    let use_width = (width - margin * 2) / col_count;
    let use_height = (height - margin * 2) / row_count;
    let sc = PxScale {
        x: use_width as f32,
        y: use_height as f32,
    };
    let font = FontRef::try_from_slice(font).unwrap();

    for row in 0..row_count {
        for col in 0..col_count {
            let t = match text.chars().nth((row * col_count + col) as usize) {
                Some(v) => v.to_string(),
                None => break,
            };
            // 获取文字实际的宽度
            let crop = ImageCrop::new(t.as_str(), use_width, 120, &font);
            let corners = crop.calculate_corners();
            let real_width = corners.1.x - corners.0.x;

            imageproc::drawing::draw_text_mut(
                &mut img,
                image::Rgba([255u8, 255u8, 255u8, 1u8]),
                (margin + col * use_width + (use_width - real_width) / 2) as i32,
                (margin + row * use_height) as i32,
                sc,
                &font,
                t.as_str(),
            );
        }
    }

    let mut buf = std::io::Cursor::new(Vec::new());

    img.write_to(&mut buf, image::ImageFormat::Jpeg).unwrap();

    Ok(buf.into_inner())
}

#[cfg(not(feature = "cover"))]
pub(crate) fn gen_cover(book_name: &str, font: &[u8]) -> IResule<Vec<u8>> {
    panic!("自动封面需要启用 plotters features")
}

#[cfg(all(test, feature = "cover"))]
mod tests {
    #[test]
    fn test_gen_cover() {
        let f = if std::path::Path::new("target").exists() {
            "target/SourceHanSansSC-Bold.otf"
        } else {
            "../target/SourceHanSansSC-Bold.otf"
        };
        let font = std::fs::read(f).or_else( |_|{
             tinyget::get("https://github.com/adobe-fonts/source-han-serif/raw/refs/heads/release/SubsetOTF/CN/SourceHanSerifCN-Bold.otf").send().map(|v|{
                let s =v.as_bytes().to_vec();
                println!("{} {:?}",s.len(),v.headers);
                if &s.len().to_string() != v.headers.get("content-length").unwrap_or(&String::new()) && v.status_code !=200 {
                    panic!("字体文件下载失败");
                }
                let _ = std::fs::write(f, s.clone());
                s
            })
        }).unwrap();

        let m = super::gen_cover("书名", &font).unwrap();
        std::fs::write(
            if std::path::Path::new("target").exists() {
                "target/cover.jpeg"
            } else {
                "../target/cover.jpeg"
            },
            m,
        )
        .unwrap();

        let m = super::gen_cover("十一个字的书名十一个字", &font).unwrap();
        std::fs::write(
            if std::path::Path::new("target").exists() {
                "target/cover2.jpeg"
            } else {
                "../target/cover2.jpeg"
            },
            m,
        )
        .unwrap();
    }
}
