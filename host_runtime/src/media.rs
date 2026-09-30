use std::io::Cursor;
use image::{ImageReader, ImageFormat}; // Заменили ImageOutputFormat на ImageFormat
use wasmtime::{Caller, Linker};

use crate::state::HostState;
use crate::utils::{read_bytes_from_mem, with_mem_slice, write_bytes_to_mem};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ImageTransform {
    pub resize_width: u32,
    pub resize_height: u32,
    pub rotate_degrees: u16,
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_w: u32,
    pub crop_h: u32,
}

pub fn register_media_api(linker: &mut Linker<HostState>) -> anyhow::Result<()> {
    // 1. Узнать размеры изображения (Width, Height) без полного декодирования
    linker.func_wrap(
        "env",
        "host_image_info",
        |mut caller: Caller<'_, HostState>,
         img_ptr: i32,
         img_len: i32,
         out_width_ptr: i32,
         out_height_ptr: i32| -> i32 {
            let img_bytes = match read_bytes_from_mem(&mut caller, img_ptr, img_len) {
                Ok(b) => b,
                Err(_) => return -1,
            };

            let Ok(reader) = ImageReader::new(Cursor::new(&img_bytes)).with_guessed_format() else {
                return -2;
            };

            let Ok((width, height)) = reader.into_dimensions() else {
                return -3;
            };

            if write_bytes_to_mem(&mut caller, out_width_ptr, &width.to_le_bytes()).is_err() {
                return -4;
            }
            if write_bytes_to_mem(&mut caller, out_height_ptr, &height.to_le_bytes()).is_err() {
                return -4;
            }

            0
        },
    )?;

    // 2. Декодировать картинку в сырой буфер RGBA
    linker.func_wrap(
        "env",
        "host_decode_image",
        |mut caller: Caller<'_, HostState>,
         img_ptr: i32,
         img_len: i32,
         out_rgba_ptr: i32,
         out_max_len: i32| -> i32 {
            let img_bytes = match read_bytes_from_mem(&mut caller, img_ptr, img_len) {
                Ok(b) => b,
                Err(_) => return -1,
            };

            let Ok(img) = image::load_from_memory(&img_bytes) else {
                return -2;
            };

            let rgba = img.to_rgba8();
            let raw_pixels = rgba.as_raw();

            if raw_pixels.len() > out_max_len as usize {
                return -3; // Буфер Гостя слишком мал
            }

            if write_bytes_to_mem(&mut caller, out_rgba_ptr, raw_pixels).is_err() {
                return -4;
            }

            raw_pixels.len() as i32
        },
    )?;

    // 3. Полный цикл транформации (Crop -> Rotate -> Resize -> JPEG Encode)
    linker.func_wrap(
        "env",
        "host_process_image",
        |mut caller: Caller<'_, HostState>,
         src_ptr: i32,
         src_len: i32,
         transform_ptr: i32,
         out_ptr: i32,
         out_max_len: i32| -> i32 {
            // Читаем параметры трансформации с учетом безопасного выравнивания (read_unaligned)
            let transform = match with_mem_slice(
                &mut caller,
                transform_ptr,
                std::mem::size_of::<ImageTransform>() as i32,
                |slice| unsafe {
                    std::ptr::read_unaligned(slice.as_ptr() as *const ImageTransform)
                },
            ) {
                Ok(t) => t,
                Err(_) => return -1,
            };

            // Читаем исходник
            let src_bytes = match read_bytes_from_mem(&mut caller, src_ptr, src_len) {
                Ok(bytes) => bytes,
                Err(_) => return -2,
            };

            // Декодируем
            let mut img = match image::load_from_memory(&src_bytes) {
                Ok(img) => img,
                Err(_) => return -3,
            };

            // Кроп
            if transform.crop_w > 0 && transform.crop_h > 0 {
                img = img.crop_imm(
                    transform.crop_x,
                    transform.crop_y,
                    transform.crop_w,
                    transform.crop_h,
                );
            }

            // Поворот
            match transform.rotate_degrees {
                90 => img = img.rotate90(),
                180 => img = img.rotate180(),
                270 => img = img.rotate270(),
                _ => {}
            }

            // Ресайз
            if transform.resize_width > 0 && transform.resize_height > 0 {
                img = img.resize_exact(
                    transform.resize_width,
                    transform.resize_height,
                    image::imageops::FilterType::Triangle,
                );
            }

            // Кодируем результат в JPEG
            let mut encoded_result = Vec::new();
                // Используем img.write_to с ImageFormat::Jpeg вместо устаревшего ImageOutputFormat
                if img.write_to(&mut Cursor::new(&mut encoded_result), ImageFormat::Jpeg).is_err() {
                    return -4;
                }

            if encoded_result.len() > out_max_len as usize {
                return -5;
            }

            if write_bytes_to_mem(&mut caller, out_ptr, &encoded_result).is_err() {
                return -6;
            }

            encoded_result.len()  as i32
        },
    )?;

    Ok(())
}