pub(crate) mod image_protocol;

use crate::error::Result;
use crate::files::FileDescriptor;
use ::image::codecs::jpeg::JpegEncoder;
use ::image::io::Reader as ImageReader;
use ::image::{ColorType, ImageEncoder};
use anyhow::Context;
use fast_image_resize as fr;
use fr::FilterType;
use image::DynamicImage;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use strum::{Display, EnumString};

#[derive(EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum ThumbnailMode {
    Cover,
    Contain,
}

pub struct ThumbnailParams {
    max_size: u32,
    mode: ThumbnailMode,
}

impl ThumbnailParams {
    pub fn from_str(max_size: &str, mode: &str) -> Result<Self> {
        let max_size = max_size
            .parse()
            .context(format!("Cannot parse {} to u32", max_size))?;
        let mode = mode
            .parse()
            .context(format!("Cannot parse {} to ThumbnailMode", mode))?;
        Ok(Self { max_size, mode })
    }

    fn cover(max_size: u32) -> Self {
        Self {
            max_size,
            mode: ThumbnailMode::Cover,
        }
    }

    fn contain(max_size: u32) -> Self {
        Self {
            max_size,
            mode: ThumbnailMode::Contain,
        }
    }
}

impl Default for ThumbnailParams {
    fn default() -> Self {
        Self {
            max_size: 1920,
            mode: ThumbnailMode::Contain,
        }
    }
}

impl Display for ThumbnailParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.max_size, self.mode)
    }
}

struct Image {
    data: DynamicImage,
}

impl Image {
    fn open(path: &str) -> Image {
        let file = ImageReader::open(path).unwrap();
        let img = file.decode().unwrap();

        Image { data: img }
    }

    fn get(&self) -> fr::Image {
        let width = NonZeroU32::new(self.data.width()).unwrap();
        let height = NonZeroU32::new(self.data.height()).unwrap();
        fr::Image::from_vec_u8(
            width,
            height,
            self.data.to_rgb8().into_raw(),
            fr::PixelType::U8x3,
        )
        .unwrap()
    }
}

struct Size {
    width: u32,
    height: u32,
}

impl Size {
    fn get_non_zero_width(&self) -> NonZeroU32 {
        NonZeroU32::new(self.width).unwrap()
    }
    fn get_non_zero_height(&self) -> NonZeroU32 {
        NonZeroU32::new(self.height).unwrap()
    }
}

pub fn generate_thumbnails(file_desc: &FileDescriptor, output_directory: &Path) -> Vec<PathBuf> {
    let mut thumbnails = Vec::new();

    let img = Image::open(&file_desc.path);
    let img = img.get();

    let cover_thumbnail = ThumbnailParams::cover(512);
    thumbnails.push(generate_thumbnail_keep_aspect(
        &img,
        file_desc,
        output_directory,
        &cover_thumbnail,
    ));

    let contain_thumbnail = ThumbnailParams::contain(1920);
    thumbnails.push(generate_thumbnail_keep_aspect(
        &img,
        file_desc,
        output_directory,
        &contain_thumbnail,
    ));

    thumbnails
}

pub fn generate_thumbnail(
    file_desc: &FileDescriptor,
    output_directory: &Path,
    thumbnail_params: &ThumbnailParams,
) -> PathBuf {
    let img = Image::open(&file_desc.path);
    let img = img.get();
    generate_thumbnail_keep_aspect(&img, file_desc, output_directory, thumbnail_params)
}

fn generate_thumbnail_keep_aspect(
    img: &fr::Image,
    file_desc: &FileDescriptor,
    output_directory: &Path,
    thumbnail_params: &ThumbnailParams,
) -> PathBuf {
    let src_view = img.view();
    let original_size = Size {
        width: src_view.width().get(),
        height: src_view.height().get(),
    };
    let calculated_size = match thumbnail_params.mode {
        ThumbnailMode::Cover => calculate_cover_size(original_size, thumbnail_params.max_size),
        ThumbnailMode::Contain => calculate_contain_size(original_size, thumbnail_params.max_size),
    };
    let dst_width = calculated_size.get_non_zero_width();
    let dst_height = calculated_size.get_non_zero_height();
    let mut dst_image = fr::Image::new(dst_width, dst_height, src_view.pixel_type());

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    let uuid = &file_desc.uuid;
    let image_thumbnails_dir = output_directory.join(uuid.to_string());
    fs::create_dir_all(&image_thumbnails_dir).unwrap();
    let thumbnail_path = image_thumbnails_dir.join(thumbnail_params.to_string());
    let thumbnail_file = File::create(&thumbnail_path).unwrap();
    let mut result_buf = BufWriter::new(thumbnail_file);

    JpegEncoder::new(&mut result_buf)
        .write_image(
            dst_image.buffer(),
            dst_width.get(),
            dst_height.get(),
            ColorType::Rgb8,
        )
        .unwrap();
    thumbnail_path
}

fn calculate_cover_size(original: Size, max: u32) -> Size {
    if original.width <= max || original.height <= max {
        return original;
    }

    let ratio = original.width as f32 / original.height as f32;

    if ratio > 1.0 {
        Size {
            width: (max as f32 * ratio) as u32,
            height: max,
        }
    } else {
        Size {
            width: max,
            height: (max as f32 / ratio) as u32,
        }
    }
}

fn calculate_contain_size(original: Size, max: u32) -> Size {
    if original.width <= max && original.height <= max {
        return original;
    }

    let ratio = original.width as f32 / original.height as f32;

    if ratio > 1.0 {
        Size {
            width: max,
            height: (max as f32 / ratio) as u32,
        }
    } else {
        Size {
            width: (max as f32 * ratio) as u32,
            height: max,
        }
    }
}
