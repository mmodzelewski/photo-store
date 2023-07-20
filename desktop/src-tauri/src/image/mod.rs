use crate::FileDesc;
use ::image::codecs::jpeg::JpegEncoder;
use ::image::io::Reader as ImageReader;
use ::image::{ColorType, ImageEncoder};
use fast_image_resize as fr;
use fr::FilterType;
use image::DynamicImage;
use std::fs::{self, File};
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::PathBuf;

struct Image {
    data: DynamicImage,
}

impl Image {
    fn open(path: &str) -> Image {
        let file = ImageReader::open(path).unwrap();
        let img = file.decode().unwrap();

        return Image { data: img };
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
        return NonZeroU32::new(self.width).unwrap();
    }
    fn get_non_zero_height(&self) -> NonZeroU32 {
        return NonZeroU32::new(self.height).unwrap();
    }
}

pub fn generate_thumbnail(file_desc: &FileDesc, folder_path: &PathBuf) -> PathBuf {
    let img = Image::open(&file_desc.path);

    let img = img.get();
    let mut src_view = img.view();
    let size = Size {
        width: 512,
        height: 512,
    };

    let dst_width = size.get_non_zero_width();
    let dst_height = size.get_non_zero_height();
    src_view.set_crop_box_to_fit_dst_size(dst_width, dst_height, None);
    let mut dst_image = fr::Image::new(dst_width, dst_height, src_view.pixel_type());

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    let uuid = &file_desc.uuid;
    let image_thumnails_folder = folder_path.join(uuid.to_string());
    fs::create_dir_all(&image_thumnails_folder).unwrap();
    let thumbnail_path = image_thumnails_folder.join(format!("{}x{}", size.width, size.height));
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
    return thumbnail_path;
}
