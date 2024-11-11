pub(crate) mod image_protocol;

use ::image::codecs::jpeg::JpegEncoder;
use ::image::io::Reader as ImageReader;
use ::image::{ColorType, ImageEncoder};
use fast_image_resize as fr;
use fr::FilterType;
use image::DynamicImage;
use std::fs::{self, File};
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use crate::files::FileDescriptor;

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
        return fr::Image::from_vec_u8(
            width,
            height,
            self.data.to_rgb8().into_raw(),
            fr::PixelType::U8x3,
        )
        .unwrap();
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

pub fn generate_thumbnails(file_desc: &FileDescriptor, folder_path: &Path) -> Vec<PathBuf> {
    let mut thumbnails = Vec::new();

    let img = Image::open(&file_desc.path);
    let img = img.get();

    thumbnails.push(generate_thumbnail_keep_aspect(
        &img,
        file_desc,
        folder_path,
        512,
        true,
    ));

    thumbnails.push(generate_thumbnail_keep_aspect(
        &img,
        file_desc,
        folder_path,
        1920,
        false,
    ));

    thumbnails
}

fn generate_thumbnail_keep_aspect(
    img: &fr::Image,
    file_desc: &FileDescriptor,
    folder_path: &Path,
    max_size: u32,
    cover: bool,
) -> PathBuf {
    let src_view = img.view();
    let original_size = Size {
        width: src_view.width().get(),
        height: src_view.height().get(),
    };
    let calculated_size = if cover {
        calculate_cover_size(original_size, max_size)
    } else {
        calculate_contain_size(original_size, max_size)
    };
    let dst_width = calculated_size.get_non_zero_width();
    let dst_height = calculated_size.get_non_zero_height();
    let mut dst_image = fr::Image::new(dst_width, dst_height, src_view.pixel_type());

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    let uuid = &file_desc.uuid;
    let image_thumnails_folder = folder_path.join(uuid.to_string());
    fs::create_dir_all(&image_thumnails_folder).unwrap();
    let cover_label = if cover { "cover" } else { "contain" };
    let thumbnail_path = image_thumnails_folder.join(format!("{}-{}", max_size, cover_label));
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
