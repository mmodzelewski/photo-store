use crate::FileDesc;
use ::image::codecs::jpeg::JpegEncoder;
use ::image::io::Reader as ImageReader;
use ::image::{ColorType, ImageEncoder};
use fast_image_resize as fr;
use fr::FilterType;
use image::DynamicImage;
use std::fs::File;
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

pub fn generate_thumbnail(file_desc: &FileDesc, folder_path: &PathBuf) {
    let img = Image::open(&file_desc.path);

    let img = img.get();
    let mut src_view = img.view();

    let dst_width = NonZeroU32::new(512).unwrap();
    let dst_height = NonZeroU32::new(512).unwrap();
    src_view.set_crop_box_to_fit_dst_size(dst_width, dst_height, None);
    let mut dst_image = fr::Image::new(dst_width, dst_height, src_view.pixel_type());

    let mut dst_view = dst_image.view_mut();

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer.resize(&src_view, &mut dst_view).unwrap();

    let uuid = &file_desc.uuid;
    let thumbnail_path = folder_path.join(uuid.to_string());
    let thumbnail_file = File::create(thumbnail_path).unwrap();
    let mut result_buf = BufWriter::new(thumbnail_file);

    JpegEncoder::new(&mut result_buf)
        .write_image(
            dst_image.buffer(),
            dst_width.get(),
            dst_height.get(),
            ColorType::Rgb8,
        )
        .unwrap();
}
